//! LLM 客户端抽象层（C4.1/C4.2）：trait + Reqwest 实现 + Mock + SSE 解析。
//!
//! 边界：**所有 HTTP 请求都在本文件**（系统能力 Rust 侧，code-standards §5）；
//! Key 仅在请求头流转，禁止进日志/错误详情（错误只带 HTTP 状态与响应前缀截断）。

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

// ---------- 请求/响应模型（OpenAI 兼容） ----------

/// 对话消息（role: system/user/assistant）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// 流式对话请求（model 由 `LlmTarget.model` 提供，请求体组装在 client 内——单一来源）。
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    /// 非流式短生成上限（主动话术用；None=不限）。仅传入时加入请求体。
    pub max_tokens: Option<u32>,
}

/// LLM 目标（三档路由的解析结果；Key 只在此流转，不落日志）。
#[derive(Debug, Clone, Default)]
pub struct LlmTarget {
    /// 档位名（日志只打这个，不打 Key/baseUrl 全文？baseUrl 非敏感可打）。
    pub tier: &'static str,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl LlmTarget {
    pub fn is_configured(&self) -> bool {
        !self.base_url.is_empty() && !self.model.is_empty()
    }
}

/// 流式增量流：每项为一段文本增量（或错误）。
pub type DeltaStream =
    std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String, AppError>> + Send>>;

/// LLM 客户端抽象（Mock 驱动下游单测，无网络依赖）。
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// 流式对话：返回增量流。
    async fn chat_stream(
        &self,
        target: &LlmTarget,
        request: ChatRequest,
    ) -> Result<DeltaStream, AppError>;

    /// 批量向量：显式校验维度（不静默截断——污染向量空间，勘误 #8）。
    async fn embed(
        &self,
        target: &LlmTarget,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, AppError>;
}

// ---------- Reqwest 实现 ----------

/// 默认客户端（rustls TLS；单例复用连接池）。
pub struct ReqwestLlmClient {
    http: reqwest::Client,
    embedding_dim: usize,
}

impl ReqwestLlmClient {
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            embedding_dim,
        }
    }

    /// OpenAI 兼容 chat/completions（stream）请求体。
    fn chat_body(target: &LlmTarget, request: &ChatRequest) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": target.model,
            "messages": request.messages,
            "stream": true,
        });
        // 主动话术等非流式短生成：仅当显式设置 max_tokens 时加入（不影响对话主链路）
        if let Some(mt) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(mt);
        }
        body
    }
    // 注：ChatRequest 不再携带 model（避免与 target.model 双源）；调用处构造时无需填 model。
}

#[async_trait]
impl LlmClient for ReqwestLlmClient {
    async fn chat_stream(
        &self,
        target: &LlmTarget,
        request: ChatRequest,
    ) -> Result<DeltaStream, AppError> {
        let url = format!("{}/chat/completions", target.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&target.api_key)
            .json(&Self::chat_body(target, &request))
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("HTTP {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            // 错误体只取前 160 字符（防日志膨胀；不包含 Key）
            let body = resp.text().await.unwrap_or_default();
            let prefix: String = body.chars().take(160).collect();
            return Err(AppError::Llm(format!("[{}] HTTP {status}: {prefix}", target.tier)));
        }

        // bytes 流 → SSE 增量流（手写分帧：eventsource-stream 备选方案，计划 §七）
        let byte_stream = resp.bytes_stream();
        let dim_check_stream = futures_util::stream::unfold(
            (byte_stream, String::new()),
            |(mut bytes, mut buffer)| async move {
                loop {
                    // 先尝试从缓冲解析一个完整 SSE event
                    if let Some(delta) = next_sse_delta(&mut buffer) {
                        return Some((Ok(delta), (bytes, buffer)));
                    }
                    match bytes.next().await {
                        Some(Ok(chunk)) => {
                            buffer.push_str(&String::from_utf8_lossy(&chunk));
                        },
                        Some(Err(e)) => {
                            return Some((
                                Err(AppError::Llm(format!("流读取失败: {e}"))),
                                (bytes, buffer),
                            ));
                        },
                        None => {
                            // 流结束：处理残余缓冲（若有完整 event）
                            let delta = next_sse_delta(&mut buffer);
                            if let Some(d) = delta {
                                return Some((Ok(d), (bytes, buffer)));
                            }
                            return None;
                        },
                    }
                }
            },
        );
        Ok(Box::pin(dim_check_stream))
    }

    async fn embed(
        &self,
        target: &LlmTarget,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, AppError> {
        let url = format!("{}/embeddings", target.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&target.api_key)
            .json(&serde_json::json!({ "model": target.model, "input": texts }))
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("HTTP {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let prefix: String = body.chars().take(160).collect();
            return Err(AppError::Llm(format!("[{}] HTTP {status}: {prefix}", target.tier)));
        }

        let payload: EmbeddingsResponse = resp.json().await.map_err(AppError::from)?;
        // 显式维度校验（勘误 #8：不静默截断——污染向量空间）
        for item in &payload.data {
            if item.embedding.len() != self.embedding_dim {
                return Err(AppError::Llm(format!(
                    "embedding 维度不匹配：模型返回 {} 维，向量库需要 {} 维。\
                     请在设置中更换 embedding 模型（当前记忆档模型：{}）",
                    item.embedding.len(),
                    self.embedding_dim,
                    target.model
                )));
            }
        }
        Ok(payload.data.into_iter().map(|d| d.embedding).collect())
    }
}

/// OpenAI 兼容 embeddings 响应体。
#[derive(Debug, Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingItem {
    embedding: Vec<f32>,
}

// ---------- SSE 解析（纯函数，单测覆盖） ----------

/// 从缓冲中取出下一个完整 SSE event 的 delta 文本。
///
/// 分帧规则：空行分隔 event；`data: ` 行取负载；`data: [DONE]` 视为流结束
/// （返回 None 前清空缓冲使后续读取自然结束）。
/// 返回 `None` 表示缓冲中没有完整 event（等待更多数据）。
pub fn next_sse_delta(buffer: &mut String) -> Option<String> {
    while let Some(pos) = buffer.find("\n\n") {
        let event: String = buffer.drain(..pos + 2).collect();
        for line in event.lines() {
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload == "[DONE]" {
                return None;
            }
            if let Ok(frame) = serde_json::from_str::<SseFrame>(payload)
                && let Some(delta) = frame.choices.into_iter().next().and_then(|c| c.delta.content)
            {
                return Some(delta);
            }
            // 非 chat 帧（如 usage 帧）忽略，继续找下一个 event
        }
    }
    None
}

/// OpenAI 流式帧（只关心 `choices[].delta.content`）。
#[derive(Debug, Deserialize)]
struct SseFrame {
    #[serde(default)]
    choices: Vec<SseChoice>,
}

#[derive(Debug, Deserialize)]
struct SseChoice {
    #[serde(default)]
    delta: SseDelta,
}

#[derive(Debug, Default, Deserialize)]
struct SseDelta {
    #[serde(default)]
    content: Option<String>,
}

// ---------- Mock（下游单测桩，无网络） ----------

/// Mock 客户端：按 base_url 决定成功（返回 canned deltas）或失败。
/// 供下游（router/distill/chat）`#[cfg(test)]` 单测使用——lib 本体不消费，
/// crate-type（cdylib/staticlib）下 pub 也不导出，故 allow dead_code。
#[allow(dead_code)]
pub struct MockLlmClient {
    /// 失败名单（url 前缀匹配）；不在名单内 → 成功。
    pub fail_urls: Vec<String>,
    /// 成功时的增量序列。
    pub deltas: Vec<String>,
    /// embed 失败模拟。
    pub fail_embed: bool,
    /// embed 返回的向量维度（默认 8；对接 1536 维虚拟表时用 `with_embed_dim`）。
    pub embed_dim: usize,
}

#[allow(dead_code)]
impl MockLlmClient {
    pub fn ok() -> Self {
        Self {
            fail_urls: vec![],
            deltas: vec!["你好".into(), "，我是".into(), "七七".into()],
            fail_embed: false,
            embed_dim: 8,
        }
    }

    pub fn failing(urls: &[&str]) -> Self {
        Self {
            fail_urls: urls.iter().map(|s| s.to_string()).collect(),
            deltas: vec![],
            fail_embed: false,
            embed_dim: 8,
        }
    }

    /// 指定 embed 维度（C7 集成测试对接真实 1536 维 vec_memory）。
    pub fn with_embed_dim(mut self, dim: usize) -> Self {
        self.embed_dim = dim;
        self
    }
}

#[allow(dead_code)]
#[async_trait]
impl LlmClient for MockLlmClient {
    // （trait impl；ok/failing 构造器在下方固有 impl，同样标注 allow）
    async fn chat_stream(
        &self,
        target: &LlmTarget,
        _request: ChatRequest,
    ) -> Result<DeltaStream, AppError> {
        if self.fail_urls.iter().any(|u| target.base_url.starts_with(u)) {
            return Err(AppError::Llm(format!("[{}] HTTP 500 Internal Server Error", target.tier)));
        }
        let deltas = self.deltas.clone();
        let stream = futures_util::stream::iter(deltas.into_iter().map(Ok::<_, AppError>));
        Ok(Box::pin(stream))
    }

    async fn embed(
        &self,
        _target: &LlmTarget,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, AppError> {
        if self.fail_embed {
            return Err(AppError::Llm("mock embed 失败".into()));
        }
        // 假向量：维度 = embed_dim，值由文本长度决定（仅测试用途）
        Ok(texts.into_iter().map(|t| vec![t.len() as f32; self.embed_dim]).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 喂入多行 SSE 文本，收集全部 delta。
    fn collect(sse: &str) -> Vec<String> {
        let mut buffer = sse.to_string();
        let mut out = Vec::new();
        while let Some(d) = next_sse_delta(&mut buffer) {
            out.push(d);
        }
        out
    }

    #[test]
    fn sse_parses_normal_delta_sequence() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\n\
                   data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}\n\n";
        assert_eq!(collect(sse), vec!["你", "好"]);
    }

    #[test]
    fn sse_done_terminates_stream() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n\
                   data: [DONE]\n\n\
                   data: {\"choices\":[{\"delta\":{\"content\":\"never\"}}]}\n\n";
        assert_eq!(collect(sse), vec!["a"]);
    }

    #[test]
    fn sse_handles_cross_chunk_frames() {
        // 帧被切在中间：两次喂入
        let mut buffer = String::from("data: {\"choices\":[{\"delta\":{\"cont");
        assert_eq!(next_sse_delta(&mut buffer), None, "不完整帧应等待");
        buffer.push_str("ent\":\"跨\"}}]}\n\n");
        assert_eq!(next_sse_delta(&mut buffer), Some("跨".into()));
    }

    #[test]
    fn sse_ignores_non_chat_frames() {
        let sse = "data: {\"usage\":{\"total_tokens\":10}}\n\n\
                   data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n";
        assert_eq!(collect(sse), vec!["ok"]);
    }

    #[test]
    fn mock_client_yields_deltas() {
        let mock = MockLlmClient::ok();
        let target = LlmTarget {
            tier: "main",
            base_url: "https://mock/v1".into(),
            api_key: "k".into(),
            model: "m".into(),
        };
        let stream = futures_executor_block_on(
            mock.chat_stream(&target, ChatRequest { messages: vec![], max_tokens: None }),
        )
        .expect("mock 成功");
        // StreamExt::next 是 async——经 block_on 桥接逐项收取
        let got = futures_executor_block_on(async move {
            let mut stream = stream;
            let mut got = String::new();
            while let Some(delta) = futures_util::StreamExt::next(&mut stream).await {
                got.push_str(&delta.expect("delta"));
            }
            got
        });
        assert_eq!(got, "你好，我是七七");
    }

    /// 极简同步执行器（测试用；避免引入 tokio 测试宏依赖差异）。
    fn futures_executor_block_on<F: std::future::Future>(fut: F) -> F::Output {
        // 测试环境用 Tauri/tokio 提供的 runtime；此处直接 block_on
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt")
            .block_on(fut)
    }
}

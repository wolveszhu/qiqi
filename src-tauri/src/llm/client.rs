//! LLM 客户端抽象层（C4.1/C4.2）：trait + Reqwest 实现 + Mock + SSE 解析。
//!
//! 边界：**所有 HTTP 请求都在本文件**（系统能力 Rust 侧，code-standards §5）；
//! Key 仅在请求头流转，禁止进日志/错误详情（错误只带 HTTP 状态与响应前缀截断）。

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

// ---------- 请求/响应模型（OpenAI 兼容） ----------

/// 多模态消息内容（D1.5 拖入图片走 vision `content` 数组）。
///
/// `untagged`：纯文本序列化为字符串，多模态序列化为 part 数组——与 OpenAI 协议一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// 纯文本（默认形态；C4 蒸馏/日记/禁忌/对话均走此）。
    Text(String),
    /// 多模态（文本 + 图片 URL part 数组）。
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// 提取用于落库/蒸馏的用户文本（Parts 仅取 text part 拼接；图片不持久化，MVP 记忆不存图）。
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.clone()),
                    ContentPart::ImageUrl { .. } => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

/// 多模态 part（OpenAI vision 协议：serde `tag="type"`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

/// 图片 URL（base64 data URL）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

/// 对话消息（role: system/user/assistant）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: MessageContent,
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

/// 一次 LLM 请求的 token 用量（接口 `usage` 优先；缺失时由调用方端侧估算兜底）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TokenUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
}

impl TokenUsage {
    pub fn total_tokens(&self) -> i64 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// 将 HTTP 错误体转为可读摘要：优先解析厂商 JSON 错误码/文案（兼容扁平 `code/message`
/// 与 OpenAI 风格嵌套 `error.{code,message}`），解析失败回退原始截断文本。
///
/// 仅用于 `[tier] HTTP {status}` 错误说明——**不含 Key/Prompt**，便于后台日志按
/// 模型/端点/错误码定位（如 402 余额不足、400 模型不存在）。
fn describe_http_error(body: &str) -> String {
    #[derive(Deserialize)]
    struct Flat {
        code: Option<serde_json::Value>,
        message: Option<String>,
        #[serde(default)]
        error: Option<serde_json::Value>,
    }
    match serde_json::from_str::<Flat>(body) {
        Ok(f) => {
            let (code, msg) = match &f.error {
                Some(serde_json::Value::Object(o)) => (
                    o.get("code").map(|v| v.to_string()),
                    o.get("message").and_then(|v| v.as_str()).map(str::to_string),
                ),
                _ => (f.code.as_ref().map(|v| v.to_string()), f.message.clone()),
            };
            let code = code.filter(|c| !c.is_empty());
            let msg = msg.filter(|m| !m.is_empty());
            match (code, msg) {
                (Some(c), Some(m)) => format!("code={c} message={m}"),
                (Some(c), None) => format!("code={c}"),
                (None, Some(m)) => format!("message={m}"),
                (None, None) => body.chars().take(160).collect(),
            }
        },
        Err(_) => body.chars().take(160).collect(),
    }
}

/// 流式事件：文本增量或流末用量帧（`stream_options.include_usage` 触发）。
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    Delta(String),
    Usage(TokenUsage),
}

/// 流式事件流：每项为一段文本增量、流末用量帧（或错误）。
pub type DeltaStream =
    std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<StreamEvent, AppError>> + Send>>;

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
    ///
    /// 携带 `stream_options.include_usage`：流末返回一次用量帧（额度记账的数据源，
    /// 见 `docs/billing/内置体验额度方案.md`）；不支持的厂商会忽略该字段，走端侧估算兜底。
    fn chat_body(target: &LlmTarget, request: &ChatRequest) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": target.model,
            "messages": request.messages,
            "stream": true,
            "stream_options": { "include_usage": true },
        });
        // 主动话术等非流式短生成：仅当显式设置 max_tokens 时加入（不影响对话主链路）
        if let Some(mt) = request.max_tokens {
            let _ = body
                .as_object_mut()
                .map(|obj| obj.insert("max_tokens".into(), serde_json::json!(mt)));
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
            // 附 model+endpoint（非敏感），并把 JSON 错误码/文案结构化——便于后台按
            // 模型/端点/错误码定位（如 402 余额不足、400 模型不存在）。不含 Key/Prompt。
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Llm(format!(
                "[{}] HTTP {status} model={} endpoint={} | {}",
                target.tier,
                target.model,
                target.base_url,
                describe_http_error(&body)
            )));
        }

        // bytes 流 → SSE 增量流（手写分帧：eventsource-stream 备选方案，计划 §七）
        let byte_stream = resp.bytes_stream();
        let dim_check_stream = futures_util::stream::unfold(
            (byte_stream, String::new()),
            |(mut bytes, mut buffer)| async move {
                loop {
                    // 先尝试从缓冲解析一个完整 SSE event
                    if let Some(event) = next_sse_event(&mut buffer) {
                        return Some((Ok(event), (bytes, buffer)));
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
                            let event = next_sse_event(&mut buffer);
                            if let Some(e) = event {
                                return Some((Ok(e), (bytes, buffer)));
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
            // 附 model+endpoint（非敏感）并结构化错误码/文案，便于按模型/端点/错误码定位。
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Llm(format!(
                "[{}] HTTP {status} model={} endpoint={} | {}",
                target.tier,
                target.model,
                target.base_url,
                describe_http_error(&body)
            )));
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

/// 从缓冲中取出下一个完整 SSE event（文本增量或流末用量帧）。
///
/// 分帧规则：空行分隔 event；`data: ` 行取负载；`data: [DONE]` 视为流结束
/// （返回 None 前清空缓冲使后续读取自然结束）。
/// 返回 `None` 表示缓冲中没有完整 event（等待更多数据）。
pub fn next_sse_event(buffer: &mut String) -> Option<StreamEvent> {
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
            if let Ok(frame) = serde_json::from_str::<SseFrame>(payload) {
                // 流末用量帧（include_usage：choices 为空 + usage 有值）——不再丢弃，
                // 额度记账以此为准（缺失时调用方端侧估算兜底）
                if let Some(usage) = frame.usage {
                    return Some(StreamEvent::Usage(TokenUsage {
                        prompt_tokens: usage.prompt_tokens,
                        completion_tokens: usage.completion_tokens,
                    }));
                }
                if let Some(delta) = frame.choices.into_iter().next().and_then(|c| c.delta.content)
                {
                    return Some(StreamEvent::Delta(delta));
                }
            }
            // 无法解析的帧忽略，继续找下一个 event
        }
    }
    None
}

/// OpenAI 流式帧（关心 `choices[].delta.content` 与流末 `usage`）。
#[derive(Debug, Default, Deserialize)]
struct SseFrame {
    #[serde(default)]
    choices: Vec<SseChoice>,
    #[serde(default)]
    usage: Option<SseUsage>,
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

/// OpenAI 用量帧（include_usage；字段缺失按 0 处理）。
#[derive(Debug, Default, Deserialize)]
struct SseUsage {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
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
    /// 流末用量帧（None = 不模拟 usage，调用方走端侧估算）。
    pub usage: Option<TokenUsage>,
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
            deltas: vec!["你好".into(), "，我是".into(), "喵喵".into()],
            usage: None,
            fail_embed: false,
            embed_dim: 8,
        }
    }

    pub fn failing(urls: &[&str]) -> Self {
        Self {
            fail_urls: urls.iter().map(|s| s.to_string()).collect(),
            deltas: vec![],
            usage: None,
            fail_embed: false,
            embed_dim: 8,
        }
    }

    /// 空响应桩：**连接正常但一个 delta 都没有**。
    ///
    /// 复现推理型模型（R1 / Qwen3 / GLM 思考模式）把 `max_tokens` 耗在
    /// `reasoning_content` 上、`delta.content` 全程为空的场景。
    pub fn empty_text() -> Self {
        Self { fail_urls: vec![], deltas: vec![], usage: None, fail_embed: false, embed_dim: 8 }
    }

    /// 指定流末用量帧（额度记账链路的单测桩）。
    pub fn with_usage(mut self, prompt: i64, completion: i64) -> Self {
        self.usage = Some(TokenUsage { prompt_tokens: prompt, completion_tokens: completion });
        self
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
        let mut items: Vec<Result<StreamEvent, AppError>> =
            deltas.into_iter().map(|d| Ok(StreamEvent::Delta(d))).collect();
        if let Some(u) = self.usage {
            items.push(Ok(StreamEvent::Usage(u)));
        }
        let stream = futures_util::stream::iter(items);
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

    /// 喂入多行 SSE 文本，收集全部事件。
    fn collect(sse: &str) -> Vec<StreamEvent> {
        let mut buffer = sse.to_string();
        let mut out = Vec::new();
        while let Some(e) = next_sse_event(&mut buffer) {
            out.push(e);
        }
        out
    }

    #[test]
    fn sse_parses_normal_delta_sequence() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\n\
                   data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}\n\n";
        assert_eq!(
            collect(sse),
            vec![StreamEvent::Delta("你".into()), StreamEvent::Delta("好".into())]
        );
    }

    #[test]
    fn sse_done_terminates_stream() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n\
                   data: [DONE]\n\n\
                   data: {\"choices\":[{\"delta\":{\"content\":\"never\"}}]}\n\n";
        assert_eq!(collect(sse), vec![StreamEvent::Delta("a".into())]);
    }

    #[test]
    fn sse_handles_cross_chunk_frames() {
        // 帧被切在中间：两次喂入
        let mut buffer = String::from("data: {\"choices\":[{\"delta\":{\"cont");
        assert_eq!(next_sse_event(&mut buffer), None, "不完整帧应等待");
        buffer.push_str("ent\":\"跨\"}}]}\n\n");
        assert_eq!(next_sse_event(&mut buffer), Some(StreamEvent::Delta("跨".into())));
    }

    #[test]
    fn sse_surfaces_usage_frame_from_include_usage() {
        // include_usage 的流末帧：choices 为空 + usage 有值 → Usage 事件（不再丢弃）
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n\
                   data: {\"choices\":[],\"usage\":{\"prompt_tokens\":120,\"completion_tokens\":30}}\n\n\
                   data: [DONE]\n\n";
        assert_eq!(
            collect(sse),
            vec![
                StreamEvent::Delta("ok".into()),
                StreamEvent::Usage(TokenUsage { prompt_tokens: 120, completion_tokens: 30 }),
            ]
        );
    }

    #[test]
    fn sse_regular_frames_with_null_usage_stay_deltas() {
        // 常规帧通常带 "usage": null → 不应误判为用量帧
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}],\"usage\":null}\n\n";
        assert_eq!(collect(sse), vec![StreamEvent::Delta("a".into())]);
    }

    #[test]
    fn chat_body_includes_stream_options_for_usage() {
        let target = LlmTarget {
            tier: "builtin",
            base_url: "https://x/v1".into(),
            api_key: "k".into(),
            model: "m".into(),
        };
        let body = ReqwestLlmClient::chat_body(
            &target,
            &ChatRequest { messages: vec![], max_tokens: None },
        );
        assert_eq!(body["stream_options"]["include_usage"], serde_json::json!(true));
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
            while let Some(event) = futures_util::StreamExt::next(&mut stream).await {
                match event.expect("event") {
                    StreamEvent::Delta(d) => got.push_str(&d),
                    StreamEvent::Usage(_) => {},
                }
            }
            got
        });
        assert_eq!(got, "你好，我是喵喵");
    }

    #[test]
    fn mock_client_yields_usage_when_configured() {
        let mock = MockLlmClient::ok().with_usage(11, 7);
        let target = LlmTarget {
            tier: "builtin",
            base_url: "https://mock/v1".into(),
            api_key: "k".into(),
            model: "m".into(),
        };
        let stream = futures_executor_block_on(
            mock.chat_stream(&target, ChatRequest { messages: vec![], max_tokens: None }),
        )
        .expect("mock 成功");
        let events = futures_executor_block_on(async move {
            futures_util::StreamExt::collect::<Vec<_>>(stream).await
        });
        let usage: Vec<TokenUsage> = events
            .into_iter()
            .filter_map(|e| match e.expect("event") {
                StreamEvent::Usage(u) => Some(u),
                StreamEvent::Delta(_) => None,
            })
            .collect();
        assert_eq!(usage, vec![TokenUsage { prompt_tokens: 11, completion_tokens: 7 }]);
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

//! 异步沉淀管道（C4.6）：对话结束后抽取事实 / 会话摘要 / 向量写入。
//!
//! 纪律：
//! - **不阻塞对话**——done 事件后 `tauri::async_runtime::spawn` 后台执行；
//! - 蒸馏走 **memory 档**廉价模型；失败重试一次后放弃（静默 + 错误日志）；
//! - JSON 解析**宽松三态**：合法数组 / markdown 围栏包裹 / 空输出均处理，非 JSON 静默跳过；
//! - 向量写入显式维度校验（embed 侧已校验），`rowid = facts.id`、`source_text = content`。
//!
//! TODO(C7 扩展锚位)：禁忌判定（Prompt 扩展一步）、日记生成、记忆编辑向量重建
//! ——复用本文件的 `DistillTask` 框架扩展（任务枚举 + 统一 embed/写库出口）。

use serde_json::Value;

use super::client::{ChatMessage, ChatRequest, LlmClient, LlmTarget};
use crate::db::repositories::{facts, vec_memory};
use crate::error::AppError;

/// 事实抽取 Prompt（要求 JSON 数组输出；单测覆盖三态解析）。
pub const FACT_EXTRACT_PROMPT: &str = "从下面的对话中提取关于用户的、值得长期记住的事实。\
只输出 JSON 数组，每个元素形如 {\"content\":\"...\",\"category\":0}。\
category：0=偏好，1=事件，2=画像。没有值得记住的内容就输出 []。只输出 JSON，不要多余文字。";

/// 解析事实抽取输出（**宽松三态**：直接 JSON / markdown 围栏 / 空输出）。
///
/// 非 JSON（模型跑偏）→ 返回空集（静默跳过，不 panic）。
/// category 越界/缺失 → 默认 0；confidence 缺失 → 0.6（计划口径）。
pub fn parse_extracted_facts(raw: &str) -> Vec<(String, i64, f64)> {
    let trimmed = raw.trim();
    let json_text = strip_code_fence(trimmed);
    if json_text.is_empty() {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(json_text) else {
        return Vec::new(); // 非 JSON：静默跳过
    };
    let Some(array) = value.as_array() else { return Vec::new() };
    array
        .iter()
        .filter_map(|item| {
            let content = item.get("content")?.as_str()?.trim().to_string();
            if content.is_empty() {
                return None;
            }
            let category = item.get("category").and_then(Value::as_i64).unwrap_or(0);
            let category = if (0..=2).contains(&category) { category } else { 0 };
            let confidence = item.get("confidence").and_then(Value::as_f64).unwrap_or(0.6);
            Some((content, category, confidence.clamp(0.0, 1.0)))
        })
        .collect()
}

/// 剥离 markdown 代码围栏（```json ... ```）。
fn strip_code_fence(raw: &str) -> &str {
    let t = raw.trim();
    if let Some(rest) = t.strip_prefix("```") {
        // 去掉语言标记行
        let rest = rest.split_once('\n').map(|(_, body)| body).unwrap_or("");
        let end = rest.rfind("```").unwrap_or(rest.len());
        return rest[..end].trim();
    }
    t
}

/// 蒸馏任务输入（C7 扩展：增加任务枚举后复用同一执行器）。
pub struct DistillInput {
    pub session_id: String,
    pub user_text: String,
    pub assistant_text: String,
}

/// 执行沉淀（在 async 上下文中调用；DB 阻塞段由调用方 spawn_blocking 包裹）。
///
/// 步骤：① facts 抽取（memory 档）→ 落库 + embed + 向量写入
/// ② 消息数超阈值 → 会话摘要 upsert。失败重试一次后放弃（返回 Err 供日志）。
pub async fn run_distill(
    client: &dyn LlmClient,
    memory_target: &LlmTarget,
    conn: &rusqlite::Connection,
    input: &DistillInput,
) -> Result<(usize, bool), AppError> {
    // ① 事实抽取（一次请求；失败重试一次）
    let raw = extract_facts(client, memory_target, &input.user_text, &input.assistant_text).await?;
    let extracted = parse_extracted_facts(&raw);

    // 落库 + 向量
    let mut written = 0usize;
    for (content, category, confidence) in &extracted {
        let fact_id = facts::insert(conn, *category, content, *confidence)?;
        let embedding = client
            .embed(memory_target, vec![content.clone()])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Llm("embed 返回空结果".into()))?;
        vec_memory::insert(conn, fact_id, &vec_memory::embedding_bytes(&embedding), content)?;
        written += 1;
    }

    // ② 会话摘要（消息数超阈值才触发）
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
            rusqlite::params![input.session_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;
    let summarized = if count >= crate::config::SUMMARY_TRIGGER_COUNT {
        let summary = summarize_session(client, memory_target, conn, &input.session_id).await?;
        crate::db::repositories::summaries::upsert(conn, &input.session_id, &summary)?;
        true
    } else {
        false
    };

    Ok((written, summarized))
}

/// 抽取事实（重试一次）。
async fn extract_facts(
    client: &dyn LlmClient,
    target: &LlmTarget,
    user_text: &str,
    assistant_text: &str,
) -> Result<String, AppError> {
    let request = ChatRequest {
        messages: vec![
            ChatMessage { role: "system".into(), content: FACT_EXTRACT_PROMPT.into() },
            ChatMessage {
                role: "user".into(),
                content: format!("用户：{user_text}\n助手：{assistant_text}"),
            },
        ],
    };
    let mut attempt = 0;
    loop {
        attempt += 1;
        match collect_full_text(client, target, request.clone()).await {
            Ok(text) => return Ok(text),
            Err(e) if attempt == 1 => {
                // 重试一次（transient 错误容忍）
                let _ = e;
                continue;
            },
            Err(e) => return Err(e),
        }
    }
}

/// 会话摘要（读最近消息拼给 memory 档）。
async fn summarize_session(
    client: &dyn LlmClient,
    target: &LlmTarget,
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<String, AppError> {
    let history = crate::db::repositories::messages::recent(
        conn,
        session_id,
        crate::config::MAX_CONTEXT_MESSAGES as i64,
    )?;
    let transcript: String = history
        .iter()
        .map(|m| {
            let role = if m.role == 0 { "用户" } else { "助手" };
            format!("{role}：{}", m.content)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: "system".into(),
                content:
                    "把下面的对话压缩成 2-3 句客观摘要（保留用户偏好与关键事件）。只输出摘要。"
                        .into(),
            },
            ChatMessage { role: "user".into(), content: transcript },
        ],
    };
    collect_full_text(client, target, request).await
}

/// 非流式取全文（蒸馏场景不需要流式——直接收拢增量）。
async fn collect_full_text(
    client: &dyn LlmClient,
    target: &LlmTarget,
    request: ChatRequest,
) -> Result<String, AppError> {
    let stream = client.chat_stream(target, request).await?;
    let mut out = String::new();
    let mut stream = std::pin::pin!(stream);
    while let Some(delta) = futures_util::StreamExt::next(&mut stream).await {
        out.push_str(&delta?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_json_array() {
        let raw = r#"[{"content":"用户讨厌吃香菜","category":0},{"content":"昨天去了公园","category":1}]"#;
        let facts = parse_extracted_facts(raw);
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].0, "用户讨厌吃香菜");
        assert_eq!(facts[0].1, 0);
        assert!((facts[0].2 - 0.6).abs() < f64::EPSILON, "缺省 confidence=0.6");
    }

    #[test]
    fn parses_markdown_fenced_json() {
        let raw = "```json\n[{\"content\":\"喜欢深夜写代码\"}]\n```";
        let facts = parse_extracted_facts(raw);
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].0, "喜欢深夜写代码");
    }

    #[test]
    fn empty_output_yields_no_facts() {
        assert!(parse_extracted_facts("").is_empty());
        assert!(parse_extracted_facts("[]").is_empty());
        assert!(parse_extracted_facts("```json\n[]\n```").is_empty());
    }

    #[test]
    fn non_json_is_silently_skipped() {
        assert!(parse_extracted_facts("我觉得没有值得记住的内容喵").is_empty());
        assert!(parse_extracted_facts("```json\n不是JSON\n```").is_empty());
    }

    #[test]
    fn dirty_items_are_filtered_or_normalized() {
        let raw = r#"[{"content":""},{"content":"有效"},{"content":"越界","category":9},{"content":"超置信","confidence":3.0}]"#;
        let facts = parse_extracted_facts(raw);
        assert_eq!(facts.len(), 3, "空 content 被过滤");
        assert_eq!(facts[1].1, 0, "category 越界归 0");
        assert!((facts[2].2 - 1.0).abs() < f64::EPSILON, "confidence 夹取 0..1");
    }
}

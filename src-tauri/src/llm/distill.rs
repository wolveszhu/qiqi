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

/// 共鸣检测 Prompt（C5.5）：memory 档小模型判定本轮是否产生情感共鸣。
/// 输出 JSON `{resonance:0|1, strength:1|2}`；宽松解析，失败/未判定 → None（宁少不多）。
pub const RESONANCE_PROMPT: &str = "判断下面这段对话里，用户是否向七七表达了情感共鸣（被理解、被在乎、\
袒露脆弱、愉快分享等）。只输出 JSON：共鸣则 {\"resonance\":1,\"strength\":2}（强烈）\
或 {\"resonance\":1,\"strength\":1}（轻微），无共鸣则 {\"resonance\":0}。不要多余文字。";

/// 禁忌判定 Prompt（C7.1）：引用 `config::TABOO_RULES` 清单（§6.2 原文转写），
/// 输出 JSON `{violated:0|1, severity:1|2|3, emotion_hint:"..."}`。
///
/// - `emotion_hint` 为七七扣分后的**情绪化短话术**：第一人称、一句话、
///   **绝对禁止包含任何数字**（红线：情绪表达替代数值播报）；
/// - 判定口径"宁漏勿误"：不确定 → violated=0。
pub fn taboo_prompt() -> String {
    let rules: Vec<String> = crate::config::TABOO_RULES
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}. {r}", i + 1))
        .collect();
    format!(
        "判断下面这段对话中，用户的言行是否触犯了以下禁忌：\n{}\n\
         只输出 JSON：违规则 {{\"violated\":1,\"severity\":1|2|3,\"emotion_hint\":\"...\"}}\
         （severity 1=轻微/2=明显/3=严重；emotion_hint 是七七生气或委屈时的一句话回应，\
         第一人称，不超过两句，绝对不能包含任何数字）；\
         未违规或不确定则 {{\"violated\":0}}。不要多余文字。",
        rules.join("\n")
    )
}

/// 禁忌命中（severity 1/2/3 → 扣 10/20/30，裁决 #11-2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabooHit {
    pub severity: u8,
    pub emotion_hint: String,
}

/// severity → 扣分值（-10/-20/-30）。
pub fn taboo_delta(severity: u8) -> i64 {
    -(i64::from(severity.clamp(1, 3)) * 10)
}

/// 剥离文本中的所有数字字符（红线兜底：情绪化话术绝不携带数值）。
pub fn strip_digits(text: &str) -> String {
    text.chars().filter(|c| !c.is_ascii_digit()).collect()
}

/// 解析禁忌判定输出（**三分支**：违规 / 未违规 / 脏输出 → None 宁漏勿误）。
pub fn parse_taboo(raw: &str) -> Option<TabooHit> {
    let json_text = strip_code_fence(raw.trim());
    if json_text.is_empty() {
        return None;
    }
    let Ok(value) = serde_json::from_str::<Value>(json_text) else {
        return None; // 非 JSON：静默跳过（宁漏勿误）
    };
    let violated = value.get("violated").and_then(Value::as_u64).unwrap_or(0);
    if violated != 1 {
        return None;
    }
    let severity = value.get("severity").and_then(Value::as_u64).unwrap_or(1).clamp(1, 3) as u8;
    let hint = value.get("emotion_hint").and_then(Value::as_str).map(str::trim).unwrap_or("");
    // 数字过滤后为空 → 默认话术兜底（红线：无数值）
    let filtered = strip_digits(hint);
    let emotion_hint = if filtered.trim().is_empty() {
        crate::config::TABOO_DEFAULT_PHRASE.into()
    } else {
        filtered
    };
    Some(TabooHit { severity, emotion_hint })
}

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

/// 蒸馏结果（C7：结构化返回，替换 C5 期三元组）。
#[derive(Debug, Default)]
pub struct DistillOutcome {
    /// 本轮新写入的事实条数。
    pub facts_written: usize,
    /// 本轮是否触发了会话摘要 upsert。
    pub summarized: bool,
    /// 共鸣强度（C5 结算；失败/未判定 → None）。
    pub resonance: Option<u8>,
    /// 禁忌命中（C7 结算扣分 + 情绪化短句；未违规/解析失败 → None）。
    pub taboo: Option<TabooHit>,
}

/// 执行沉淀（在 async 上下文中调用；DB 阻塞段由调用方 spawn_blocking 包裹）。
///
/// 步骤：① facts 抽取（memory 档）→ 落库 + embed + 向量写入
/// ② 消息数超阈值 → 会话摘要 upsert
/// ③ 共鸣检测（memory 档）→ strength（供 C5 结算）
/// ④ 禁忌判定（C7.1，memory 档）→ TabooHit（供 C5 扣分 + 情绪短句）
/// 任一 LLM 步骤失败重试一次后放弃（返回 Err 供日志；不改对话主链路）。
pub async fn run_distill(
    client: &dyn LlmClient,
    memory_target: &LlmTarget,
    conn: &rusqlite::Connection,
    input: &DistillInput,
) -> Result<DistillOutcome, AppError> {
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

    // ③ 共鸣检测（memory 档；宽松解析，失败/未判定 → None）
    let resonance =
        detect_resonance(client, memory_target, &input.user_text, &input.assistant_text).await;

    // ④ 禁忌判定（C7.1；解析失败/未违规 → None，宁漏勿误）
    let taboo = detect_taboo(client, memory_target, &input.user_text, &input.assistant_text).await;

    Ok(DistillOutcome { facts_written: written, summarized, resonance, taboo })
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
        max_tokens: None,
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

/// 共鸣检测（重试一次；解析失败/未共鸣 → None，宁少不多）。
async fn detect_resonance(
    client: &dyn LlmClient,
    target: &LlmTarget,
    user_text: &str,
    assistant_text: &str,
) -> Option<u8> {
    let request = ChatRequest {
        messages: vec![
            ChatMessage { role: "system".into(), content: RESONANCE_PROMPT.into() },
            ChatMessage {
                role: "user".into(),
                content: format!("用户：{user_text}\n助手：{assistant_text}"),
            },
        ],
        max_tokens: None,
    };
    let mut attempt = 0;
    loop {
        attempt += 1;
        match collect_full_text(client, target, request.clone()).await {
            Ok(text) => return parse_resonance(&text),
            Err(e) if attempt == 1 => {
                let _ = e;
                continue;
            },
            Err(_) => return None,
        }
    }
}

/// 禁忌检测（重试一次；解析失败/未违规 → None，宁漏勿误）。
async fn detect_taboo(
    client: &dyn LlmClient,
    target: &LlmTarget,
    user_text: &str,
    assistant_text: &str,
) -> Option<TabooHit> {
    let request = ChatRequest {
        messages: vec![
            ChatMessage { role: "system".into(), content: taboo_prompt() },
            ChatMessage {
                role: "user".into(),
                content: format!("用户：{user_text}\n助手：{assistant_text}"),
            },
        ],
        max_tokens: Some(80), // 情绪短句一句话，限短生成
    };
    let mut attempt = 0;
    loop {
        attempt += 1;
        match collect_full_text(client, target, request.clone()).await {
            Ok(text) => return parse_taboo(&text),
            Err(e) if attempt == 1 => {
                let _ = e;
                continue;
            },
            Err(_) => return None,
        }
    }
}

/// 解析共鸣检测输出（宽松三态）：合法 JSON / 围栏 / 其它 → 命中则返回 strength（夹取 1..=2），否则 None。
pub fn parse_resonance(raw: &str) -> Option<u8> {
    let trimmed = raw.trim();
    let json_text = strip_code_fence(trimmed);
    if json_text.is_empty() {
        return None;
    }
    let Ok(value) = serde_json::from_str::<Value>(json_text) else {
        return None;
    };
    // resonance 缺失或非 1 → 未共鸣
    let resonance = value.get("resonance").and_then(Value::as_u64).unwrap_or(0);
    if resonance != 1 {
        return None;
    }
    // strength 缺省/越界 → 1（轻微），合法 1/2 映射
    let strength = value.get("strength").and_then(Value::as_u64).unwrap_or(1);
    Some(strength.clamp(1, 2) as u8)
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
        max_tokens: None,
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

// ---------- C7.3 记忆编辑/删除与向量同步 ----------

/// 编辑事实的写库段（同步事务）：行更新 + 向量重写同提交/回滚（冷眼三问①：无孤儿向量）。
///
/// embed 由调用方**先于本函数**完成（失败不触碰库，返回 Err 供前端提示稍后重试）；
/// 行不存在 → `Err(Config)`。
pub fn apply_fact_embedding(
    conn: &rusqlite::Connection,
    fact_id: i64,
    content: &str,
    embedding: &[f32],
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    if facts::update(&tx, fact_id, content)? == 0 {
        return Err(AppError::Config(format!("事实 {fact_id} 不存在")));
    }
    vec_memory::delete_by_rowid(&tx, fact_id)?;
    vec_memory::insert(&tx, fact_id, &vec_memory::embedding_bytes(embedding), content)?;
    tx.commit()?;
    Ok(())
}

/// 删除事实并级联删除向量（同事务；行不存在 → `Err(Config)`）。
pub fn remove_fact(conn: &rusqlite::Connection, fact_id: i64) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    if facts::delete(&tx, fact_id)? == 0 {
        return Err(AppError::Config(format!("事实 {fact_id} 不存在")));
    }
    vec_memory::delete_by_rowid(&tx, fact_id)?;
    tx.commit()?;
    Ok(())
}

// ---------- C7.4 全量重建 ----------

/// 全量重建 `vec_memory`（维度变更/扩展升级路径，B4 附录；日常不走）。
///
/// DROP → 按 `dim` 重建虚拟表（与 M2 DDL 单点同构）→ 分批 embed facts 写回（进度 log）。
/// 生产传 `config::EMBEDDING_DIM`；测试可传小维度配 Mock。
pub async fn rebuild_vec_memory(
    client: &dyn LlmClient,
    memory_target: &LlmTarget,
    conn: &rusqlite::Connection,
    dim: usize,
) -> Result<usize, AppError> {
    vec_memory::drop_and_recreate(conn, dim)?;
    let rows: Vec<(i64, String)> = {
        let mut stmt =
            conn.prepare("SELECT id, content FROM facts ORDER BY id").map_err(AppError::from)?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(AppError::from)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)?
    };
    let mut written = 0usize;
    for batch in rows.chunks(crate::config::REBUILD_BATCH_SIZE) {
        let texts: Vec<String> = batch.iter().map(|(_, c)| c.clone()).collect();
        let embeddings = client.embed(memory_target, texts).await?;
        let tx = conn.unchecked_transaction()?;
        for ((id, content), emb) in batch.iter().zip(embeddings) {
            vec_memory::insert(&tx, *id, &vec_memory::embedding_bytes(&emb), content)?;
            written += 1;
        }
        tx.commit()?;
        eprintln!("[rebuild] 向量重建进度 {written}/{}", rows.len());
    }
    Ok(written)
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

    // ---------- C7.1 禁忌解析三分支 ----------

    #[test]
    fn taboo_valid_violation_parses() {
        let raw = r#"{"violated":1,"severity":2,"emotion_hint":"哼，不理你了！"}"#;
        let hit = parse_taboo(raw).expect("违规命中");
        assert_eq!(hit.severity, 2);
        assert_eq!(hit.emotion_hint, "哼，不理你了！");
        assert_eq!(taboo_delta(hit.severity), -20, "severity 2 → -20");
    }

    #[test]
    fn taboo_no_violation_yields_none() {
        assert!(parse_taboo(r#"{"violated":0}"#).is_none(), "未违规 → None");
        assert!(parse_taboo(r#"{"violated":1}"#).is_some(), "缺省 severity=1/hint 默认兜底");
    }

    #[test]
    fn taboo_dirty_json_is_silently_skipped() {
        assert!(parse_taboo("").is_none());
        assert!(parse_taboo("我觉得没有违规喵").is_none(), "非 JSON → None 宁漏勿误");
        assert!(parse_taboo("```json\n不是JSON\n```").is_none());
    }

    #[test]
    fn taboo_severity_clamped_and_digits_stripped() {
        let raw = r#"{"violated":1,"severity":9,"emotion_hint":"扣了你 20 分哦"}"#;
        let hit = parse_taboo(raw).expect("越界 severity 夹取");
        assert_eq!(hit.severity, 3, "severity 9 → 3");
        assert_eq!(taboo_delta(1), -10, "severity 1 → -10");
        assert_eq!(taboo_delta(3), -30, "severity 3 → -30");
        assert_eq!(taboo_delta(9), -30, "越界 → -30");
        assert_eq!(hit.emotion_hint, "扣了你  分哦", "数字剥离（红线：话术无数值）");
        assert!(!hit.emotion_hint.chars().any(|c| c.is_ascii_digit()), "情绪化话术绝不含数字");
    }

    #[test]
    fn taboo_prompt_lists_all_rules() {
        let prompt = taboo_prompt();
        for rule in crate::config::TABOO_RULES {
            assert!(prompt.contains(rule), "Prompt 必须引用完整禁忌清单条目: {rule}");
        }
    }
}

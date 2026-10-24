//! 日记生成（C7.2）：每日 00:05 定时 + 启动补漏 + 手动重生成。
//!
//! 裁决 #11-4：① 每日 00:05 后台定时（读昨日 messages）② 启动补漏
//! （缺哪天补哪天，上限近 7 天）③ 手动 `memory_regenerate_diary`（upsert 覆盖）。
//! 不做 idle 长离开即时触发（登记 E1 评估）。
//!
//! 三段式拆分（collect → generate → store）：DB 锁不跨 await（冷眼三问①并发安全）；
//! 生成**复用主对话编排链**（自定义 → 内置 → 三档）完整 failover；链全失败则静默跳过（只 log）。

use chrono::TimeZone;
use serde_json::Value;

use super::client::{ChatMessage, ChatRequest, LlmClient};
use super::router::{self, ModelChain};
use crate::db::repositories::{diaries, messages};
use crate::error::AppError;

/// 日记生成 Prompt：第一人称"猫猫日记"，输出 JSON `{content, mood}`。
///
/// mood：0=开心 1=平静 2=低落（DiaryMood，D3 看板消费）；只输出 JSON。
/// `name` 为用户设定的宠物名（B 阶段：来自 `pet_states.pet_name`，非硬编码）。
pub fn build_diary_prompt(name: &str) -> String {
    format!(
        "你是「{name}」，一只住在用户电脑桌面的猫咪。\
根据下面的对话记录，以第一人称写一篇当天的日记（像猫猫的碎碎念，2-4 句，可爱、口语化）。\
只输出 JSON：{{\"content\":\"日记正文\",\"mood\":0}}，mood 取 0=开心/1=平静/2=低落。\
不要多余文字，不要包含任何数字、统计或量化信息，只用叙述。",
        name = name
    )
}

/// 日记草稿（解析结果）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiaryDraft {
    pub content: String,
    pub mood: Option<i64>,
}

/// 解析日记输出（宽松三态）：合法 JSON / 围栏 / 非 JSON（整段作正文，mood 空）/ 空 → None。
pub fn parse_diary(raw: &str) -> Option<DiaryDraft> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let json_text = strip_fence(trimmed);
    if let Ok(value) = serde_json::from_str::<Value>(json_text) {
        let content =
            value.get("content").and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
        if content.is_empty() {
            return None;
        }
        let mood = value.get("mood").and_then(Value::as_i64);
        let mood = mood.filter(|m| (0..=2).contains(m));
        return Some(DiaryDraft { content, mood });
    }
    // 非 JSON（模型跑偏）：整段作为日记正文（mood 未知）
    Some(DiaryDraft { content: trimmed.to_string(), mood: None })
}

/// 剥离 markdown 代码围栏（与 distill 同规则；独立小实现避免跨域私有依赖）。
fn strip_fence(raw: &str) -> &str {
    let t = raw.trim();
    if let Some(rest) = t.strip_prefix("```") {
        let rest = rest.split_once('\n').map(|(_, body)| body).unwrap_or("");
        let end = rest.rfind("```").unwrap_or(rest.len());
        return rest[..end].trim();
    }
    t
}

/// 读指定日期的对话并拼装日记输入（本地时区；无消息 → None 跳过当日）。
/// `pet_name` 为用户设定的宠物名（B 阶段：对话记录里的助手角色名跟随它）。
pub fn collect_diary_input(
    conn: &rusqlite::Connection,
    date: &str,
    pet_name: &str,
) -> Result<Option<String>, AppError> {
    let rows = messages::by_date(conn, date, crate::config::DIARY_MAX_MESSAGES)?;
    if rows.is_empty() {
        return Ok(None);
    }
    let mut transcript = rows
        .iter()
        .map(|m| {
            let role = if m.role == 0 { "用户" } else { pet_name };
            format!("{role}：{}", m.content)
        })
        .collect::<Vec<_>>()
        .join("\n");
    // 超长截断（防 token 爆炸；按字符边界）
    let max = crate::config::DIARY_MAX_INPUT_CHARS;
    if transcript.chars().count() > max {
        transcript = transcript.chars().take(max).collect();
    }
    Ok(Some(transcript))
}

/// 生成日记文本（沿编排链 failover；输出经宽松解析）。
///
/// 目标级重试由 `chat_text_with_chain` 的链路 failover 承担，此处只解析一次——
/// 解析失败（`parse_diary` 仅在输出为空时返回 None）→ `Err`，由调用方静默跳过。
pub async fn generate_diary_text(
    client: &dyn LlmClient,
    chain: &ModelChain,
    transcript: &str,
    pet_name: &str,
) -> Result<DiaryDraft, AppError> {
    let request = ChatRequest {
        messages: vec![
            ChatMessage { role: "system".into(), content: build_diary_prompt(pet_name).into() },
            ChatMessage { role: "user".into(), content: transcript.to_string().into() },
        ],
        max_tokens: Some(300),
    };
    let outcome = router::chat_text_with_chain(client, chain, request).await?;
    parse_diary(&outcome.text).ok_or_else(|| AppError::Llm("日记输出为空，无法解析".into()))
}

/// 落库（upsert 覆盖当日；裁决 #11-5）。
pub fn store_diary(
    conn: &rusqlite::Connection,
    date: &str,
    draft: &DiaryDraft,
) -> Result<i64, AppError> {
    diaries::upsert(conn, date, &draft.content, draft.mood)
}

/// 启动补漏：近 `backfill_days` 天内"有对话但无日记"的日期（倒序，最近优先）。
pub fn missing_diary_dates(
    conn: &rusqlite::Connection,
    today: chrono::NaiveDate,
    backfill_days: i64,
) -> Result<Vec<String>, AppError> {
    let mut missing = Vec::new();
    for offset in 1..=backfill_days {
        let Some(day) = today.checked_sub_signed(chrono::Duration::days(offset)) else {
            continue;
        };
        let s = day.format("%Y-%m-%d").to_string();
        if messages::count_by_date(conn, &s)? > 0 && !diaries::exists(conn, &s)? {
            missing.push(s);
        }
    }
    Ok(missing)
}

/// 下一次定时触点距现在的时长（本地 00:05；已过则明日）。
fn next_cron_in(now: chrono::DateTime<chrono::Local>) -> std::time::Duration {
    let naive = now
        .date_naive()
        .and_hms_opt(crate::config::DIARY_CRON_HOUR, crate::config::DIARY_CRON_MINUTE, 0)
        .unwrap_or_else(|| now.naive_local());
    let mut next =
        chrono::Local.from_local_datetime(&naive).single().unwrap_or_else(chrono::Local::now);
    if next <= now {
        next += chrono::Duration::days(1);
    }
    (next - now).to_std().unwrap_or(std::time::Duration::from_secs(60))
}

/// 为指定日期生成并落库（collect → generate → store 三段式；锁不跨 await）。
pub async fn generate_for_date(app: &tauri::AppHandle, date: &str) -> Result<bool, AppError> {
    use tauri::Manager as _;
    let state = app.state::<crate::state::AppState>();

    // 休眠期暂停后台日记（用户决策：边缘吸附休眠期间彻底静默，省 token 与后台活动）。
    // 补漏与每日定时两条路径都经本函数，一处门控全覆盖。
    if state.is_sleep_mode() {
        eprintln!("[diary] 休眠中，跳过 {date} 生成");
        return Ok(false);
    }

    // ① 配置快照 + 编排链 + 读输入（锁段一）
    let (chain, transcript, pet_name) = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        let tiered = crate::db::repositories::app_settings::get_tiered(&conn)?;
        let quota = crate::db::repositories::app_settings::get_quota_mode(&conn)?;
        let embedded = quota == crate::config::QUOTA_MODE_EMBEDDED;
        // 与主对话同构：自定义（用户自配，优先）始终入链；内置仅在「内置额度」模式兜底
        let custom = crate::db::repositories::custom_model_settings::list_enabled_llm(&conn)?;
        let builtin = if embedded {
            crate::db::repositories::default_model_settings::list_available_llm(&conn)?
        } else {
            Vec::new()
        };
        let (cursor, last_ok) = router::next_builtin_slot();
        let chain = router::build_chain(&router::ChainInput {
            custom: &custom,
            builtin: &builtin,
            builtin_cursor: cursor,
            builtin_last_ok: last_ok,
            tiered: &tiered,
        });
        // 宠物名（B 阶段：跟随用户设置；空则 DEFAULT_PET_NAME 回退）
        let pet_name = crate::db::repositories::pet_states::get(&conn)?.pet_name;
        // 借用检查：transcript 拷贝出锁段
        match collect_diary_input(&conn, date, &pet_name)? {
            Some(t) => (chain, t, pet_name),
            None => return Ok(false), // 当日无对话：跳过（风险对策）
        }
    }; // conn guard drop

    // 链为空（自定义/内置/三档均未配置）→ 跳过多余请求
    if chain.targets.is_empty() {
        eprintln!("[diary] 编排链为空（自定义/内置/三档均未配置）；跳过 {date} 生成");
        return Ok(false);
    }

    // ② LLM 生成（无锁段；沿链 failover）
    let client = super::client::ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);
    let draft = generate_diary_text(&client, &chain, &transcript, &pet_name).await?;

    // ③ 落库（锁段二）
    {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        store_diary(&conn, date, &draft)?;
    }
    eprintln!("[diary] {date} 日记已生成（mood={:?}）", draft.mood);
    Ok(true)
}

/// 启动补漏：近 N 天缺失日记逐日补齐（有对话才补；失败仅 log 继续）。
async fn backfill(app: &tauri::AppHandle) {
    use tauri::Manager as _;
    let state = app.state::<crate::state::AppState>();
    let dates = {
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[diary] 启动补漏取锁失败: {e}");
                return;
            },
        };
        match missing_diary_dates(
            &conn,
            chrono::Local::now().date_naive(),
            crate::config::DIARY_BACKFILL_DAYS,
        ) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[diary] 启动补漏查询失败: {e}");
                return;
            },
        }
    };
    if dates.is_empty() {
        return;
    }
    eprintln!("[diary] 启动补漏 {} 天: {:?}", dates.len(), dates);
    for date in dates {
        if let Err(e) = generate_for_date(app, &date).await {
            eprintln!("[diary] 补漏 {date} 失败（跳过）: {e}");
        }
    }
}

/// 日记调度器单例（spawn 防重复；冷眼三问②）。
static SCHEDULER_STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// 启动日记调度：立即补漏 + 每日 00:05 生成昨日日记（长驻 async 任务）。
pub fn spawn_scheduler(app: tauri::AppHandle) {
    if SCHEDULER_STARTED.set(()).is_err() {
        return; // 已启动：幂等
    }
    tauri::async_runtime::spawn(async move {
        backfill(&app).await;
        loop {
            let wait = next_cron_in(chrono::Local::now());
            tokio::time::sleep(wait).await;
            // 00:05 生成"昨日"日记（当日刚过零点，昨日为完整一天）
            let yesterday = (chrono::Local::now().date_naive() - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string();
            if let Err(e) = generate_for_date(&app, &yesterday).await {
                eprintln!("[diary] 定时生成 {yesterday} 失败: {e}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_diary_json() {
        let raw = r#"{"content":"今天主人夸了我喵！","mood":0}"#;
        let d = parse_diary(raw).expect("合法 JSON");
        assert_eq!(d.content, "今天主人夸了我喵！");
        assert_eq!(d.mood, Some(0));
    }

    #[test]
    fn parses_fenced_json_and_clamps_mood() {
        let raw = "```json\n{\"content\":\"平静的一天\",\"mood\":5}\n```";
        let d = parse_diary(raw).expect("围栏 JSON");
        assert_eq!(d.content, "平静的一天");
        assert_eq!(d.mood, None, "mood 越界 → None（宁缺勿错）");
    }

    #[test]
    fn non_json_falls_back_to_plain_text() {
        let d = parse_diary("今天什么都不想做，趴着。").expect("纯文本回退");
        assert_eq!(d.content, "今天什么都不想做，趴着。");
        assert_eq!(d.mood, None);
    }

    #[test]
    fn empty_output_is_none() {
        assert!(parse_diary("").is_none());
        assert!(parse_diary("```json\n{}\n```").is_none(), "空 content → None");
    }
}

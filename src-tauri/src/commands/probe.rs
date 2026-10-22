//! probe 命令域（薄层）：C2 暴露 cursor 快照；C6 加 idle/foreground 快照 + 主动话术生成。
//!
//! 说明：cursor 实时值已由 `probe:cursor:move` 事件广播（窗口 A 的 lookAt 消费），
//! 本命令供 D1/D2 等**非订阅场景**读取最近一次坐标/状态（读缓存，零轮询）。
//!
//! 隐私红线：前台探针只取进程名（Rust 侧已保证），本文件不触碰任何窗口内容。

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::db::models::TieredValue;
use crate::db::repositories::{app_settings, pet_states};
use crate::error::AppError;
use crate::llm::client::{ChatMessage, ChatRequest, ReqwestLlmClient};
use crate::llm::prompt::{ConversationCtx, RelationView, RetrievedContext, build_system_prompt};
use crate::llm::router::chat_with_fallback;
use crate::state::AppState;

/// 读取最近一次 cursor 坐标（屏幕坐标；未轮询到则为 0,0）。
#[tauri::command]
pub fn probe_get_cursor(state: State<'_, AppState>) -> Result<(i32, i32), AppError> {
    let slot = state.cursor.lock().map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?;
    Ok(*slot)
}

/// 探针快照（一次读全量缓存；Rust 侧轮询结果，零额外系统调用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeSnapshot {
    /// 鼠标屏幕坐标（逻辑像素）。
    pub cursor: (i32, i32),
    /// 当前空闲时长（毫秒）。
    pub idle_ms: u64,
    /// 是否处于 idle 状态（≥`IDLE_THRESHOLD_MS`）。
    pub idle: bool,
    /// 前台进程名（仅进程名；null 表示无前台/桌面）。
    pub foreground: Option<String>,
}

/// 读取探针全量快照（非订阅场景：设置面板/调试用）。
#[tauri::command]
pub fn probe_get_snapshot(state: State<'_, AppState>) -> Result<ProbeSnapshot, AppError> {
    let cursor = *state.cursor.lock().map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?;
    let idle =
        *state.idle_active.lock().map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?;
    let foreground = state
        .last_foreground
        .lock()
        .map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?
        .clone();
    // 空闲时长实时计算（GetLastInputInfo 廉价）；失败回退用状态机值推断
    let idle_ms = crate::probe::idle::idle_ms().unwrap_or(if idle {
        crate::config::IDLE_THRESHOLD_MS
    } else {
        0
    });
    Ok(ProbeSnapshot { cursor, idle_ms, idle, foreground })
}

/// 主动话术生成（C6.4）：Rust 复用 C4 管道做**非流式短生成**，emit `proactive:show`
/// （负载结构 `llm::ProactiveShow`，与 C7 禁忌短句共用管道）。
///
/// - 用户未开口，由桌宠主动找话；max_tokens 限短句（max_tokens=60）；
/// - 傲娇优先：距上次互动 ≥3 天 → tsundere 话术；
/// - 全部档位失败 → 本地兜底话术（静默回落，不阻塞、不报错）；
/// - 无有效文本 → 静默放弃（下个 tick 再抽签）。
#[tauri::command]
pub async fn proactive_generate(
    app: AppHandle,
    state: State<'_, AppState>,
    tsundere: Option<bool>,
) -> Result<(), AppError> {
    let client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);

    // 配置快照（每次实时读取——改配置即生效）
    let (tiered, quota_embedded, relation_level) = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        let tiered: TieredValue = app_settings::get_tiered(&conn)?;
        let quota = app_settings::get_quota_mode(&conn)?;
        let pet = pet_states::get(&conn)?;
        (tiered, quota == crate::config::QUOTA_MODE_EMBEDDED, pet.relationship_level)
    };

    let tsundere = tsundere.unwrap_or(false);
    // 系统 Prompt：人设 + 等级指令 + 傲娇基调（复用 C4 prompt 单一真源）
    let ctx = ConversationCtx { tsundere, slow_reply: false };
    let view = RelationView { level: relation_level, pet_name: crate::config::PET_NAME };
    let mut system = build_system_prompt(&RetrievedContext::default(), &view, &ctx);
    system.push_str("\n\n现在由你主动找用户说话（不是用户先开口）。用第一人称，不超过两句话，自然可爱，像突然想起来要搭话。");
    if tsundere {
        system.push_str("你有点傲娇：先小小抱怨一下\"你怎么才回来\"，再自然地黏上去。");
    }

    let request = ChatRequest {
        messages: vec![
            ChatMessage { role: "system".into(), content: system.into() },
            ChatMessage {
                role: "user".into(), content: "（轮到你主动开口了，说点什么吧）".into()
            },
        ],
        max_tokens: Some(60),
    };

    let (stream_result, chain_log) =
        chat_with_fallback(&client, &tiered, quota_embedded, request).await;
    for entry in &chain_log {
        eprintln!("[proactive] {entry}");
    }

    let text = match stream_result {
        Ok(stream) => {
            let mut full = String::new();
            let mut stream = std::pin::pin!(stream);
            while let Some(delta) = futures_util::StreamExt::next(&mut stream).await {
                match delta {
                    Ok(d) => full.push_str(&d),
                    Err(_) => break, // 流中断：用已收集文本
                }
            }
            full
        },
        // 全档位失败：本地兜底话术（Err 携带文本）
        Err(phrase) => phrase,
    };

    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(()); // 无话可说，静默放弃
    }

    let session_id = format!("proactive-{}", chrono::Local::now().timestamp_millis());
    let mood = if tsundere { "tsundere" } else { "happy" };
    let _ = app
        .emit("proactive:show", crate::llm::ProactiveShow { session_id, text, mood: mood.into() });
    Ok(())
}

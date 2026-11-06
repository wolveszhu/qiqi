//! probe 命令域（薄层）：C2 暴露 cursor 快照；C6 加 idle/foreground 快照 + 主动话术生成。
//!
//! 说明：cursor 实时值已由 `probe:cursor:move` 事件广播（窗口 A 的 lookAt 消费），
//! 本命令供 D1/D2 等**非订阅场景**读取最近一次坐标/状态（读缓存，零轮询）。
//!
//! 隐私红线：前台探针只取进程名（Rust 侧已保证），本文件不触碰任何窗口内容。

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::db::models::TieredValue;
use crate::db::repositories::{
    app_settings, custom_model_settings, default_model_settings, pet_states,
};
use crate::error::AppError;
use crate::llm::client::{ChatMessage, ChatRequest, ReqwestLlmClient};
use crate::llm::prompt::{ConversationCtx, RelationView, RetrievedContext, build_system_prompt};
use crate::llm::router;
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

    // 配置快照（每次实时读取——改配置即生效）+ 两个模型源（自定义优先、内置兜底）
    let (tiered, _quota_embedded, relation_level, pet_name, custom, builtin) = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        let tiered: TieredValue = app_settings::get_tiered(&conn)?;
        let quota = app_settings::get_quota_mode(&conn)?;
        let pet = pet_states::get(&conn)?;
        let embedded = quota == crate::config::QUOTA_MODE_EMBEDDED;
        // 内置额度模式才接入内置模型兜底；BYOK 模式仅自定义 + 三档
        let builtin =
            if embedded { default_model_settings::list_available_llm(&conn)? } else { Vec::new() };
        (
            tiered,
            embedded,
            pet.relationship_level,
            pet.pet_name,
            custom_model_settings::list_enabled_llm(&conn)?,
            builtin,
        )
    };

    let tsundere = tsundere.unwrap_or(false);
    // 系统 Prompt：人设 + 等级指令 + 傲娇基调（复用 C4 prompt 单一真源）
    let ctx = ConversationCtx { tsundere, slow_reply: false };
    let view = RelationView { level: relation_level, pet_name };
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

    // 额度记账兜底口径：请求全字符数（request 即将被链消费，先取）
    let prompt_chars = crate::llm::quota::request_chars(&request);
    let (stream_result, chain_log, hit) =
        router::chat_orchestrated(&client, &custom, &builtin, &tiered, request).await;
    for entry in &chain_log {
        eprintln!("[proactive] {entry}");
    }

    let mut usage = None;
    let mut stream_interrupted: Option<String> = None;
    let text = match stream_result {
        Ok(stream) => {
            let mut full = String::new();
            let mut stream = std::pin::pin!(stream);
            while let Some(event) = futures_util::StreamExt::next(&mut stream).await {
                match event {
                    Ok(crate::llm::client::StreamEvent::Delta(d)) => full.push_str(&d),
                    Ok(crate::llm::client::StreamEvent::Usage(u)) => usage = Some(u),
                    Err(e) => {
                        // 流中断：用已收集文本；记账记失败条目（0 点）
                        stream_interrupted = Some(e.to_string());
                        break;
                    },
                }
            }
            full
        },
        // 全档位失败：本地兜底话术（Err 携带文本）
        Err(phrase) => phrase,
    };

    // 记账（后台任务：失败仅 log，不中断话术）
    if let Some(h) = &hit {
        let chars =
            crate::llm::quota::CallChars { prompt: prompt_chars, completion: text.chars().count() };
        let charge_result =
            state.db.lock().map_err(|e| AppError::Db(format!("{e}"))).and_then(|conn| {
                crate::llm::quota::record_call(
                    &conn,
                    "proactive",
                    Some((h.tier.as_str(), h.model.as_str(), h.platform.as_str())),
                    usage,
                    chars,
                    stream_interrupted.is_none(),
                    stream_interrupted.as_deref(),
                )
            });
        if let Err(e) = charge_result {
            eprintln!("[quota] 主动话术记账失败: {e}");
        }
    }

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

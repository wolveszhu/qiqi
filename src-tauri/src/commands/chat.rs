//! chat 命令域（C4.5 实装，替换 B5 echo 桩——接口签名与事件名不变，前端零改动）。
//!
//! 数据流（计划 §三）：
//! user 落库 → spawn_blocking 混合检索（embed + KNN + facts）→ prompt 拼装
//! → router 回落链流式 → emit delta → done → assistant 落库 → spawn 蒸馏。
//!
//! 隐私红线：日志禁打 Key 与 Prompt 全文（只打长度与档位）；Prompt 载荷
//! 仅含对话文本与本地记忆，绝不含屏幕内容/进程窗口内容。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use tauri::async_runtime;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::repositories::{messages, vec_memory};
use crate::error::AppError;
use crate::llm::client::{ChatMessage, ChatRequest, LlmClient, LlmTarget, ReqwestLlmClient};
use crate::llm::prompt::{RelationView, RetrievedContext, build_system_prompt};
use crate::llm::router::chat_with_fallback;
use crate::state::AppState;

/// 会话取消标志注册表（per-session；abort 命令置位，流循环检查）。
/// 模块级 OnceLock：会话表生命周期与应用一致，AppState 不为此扩字段。
fn abort_registry() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 流式事件负载（与前端 DTO 对齐，camelCase）。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamDelta {
    session_id: String,
    delta: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamDone {
    session_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamError {
    session_id: String,
    kind: String,
    message: String,
}

/// 发送对话（流式；命令立即返回，结果经事件推送）。
#[tauri::command]
pub async fn chat_send_stream(
    app: AppHandle,
    session_id: String,
    text: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);

    // ① 配置快照（每次实时读取——改配置即生效，冷眼三问③）
    let (tiered, quota_embedded, relation) = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        let tiered = app_settings_tiered(&conn)?;
        let quota = app_settings_quota(&conn)?;
        let pet = crate::db::repositories::pet_states::get(&conn)?;
        (
            tiered,
            quota == crate::config::QUOTA_MODE_EMBEDDED,
            (pet.relationship_level, pet.pet_name),
        )
    };

    // ② user 消息落库（阻塞段）
    {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        messages::insert(&conn, &session_id, 0, &text)?;
    }

    // ③ 混合检索（spawn_blocking：embed + KNN + facts 近因）
    let retrieval_client: Arc<dyn LlmClient> = Arc::new(client);
    let memory_target = resolve_memory_target(&tiered, quota_embedded);
    let session_for_retrieval = session_id.clone();
    let text_for_retrieval = text.clone();
    let app_for_retrieval = app.clone();
    let retrieved = async_runtime::spawn_blocking(move || -> Result<RetrievedContext, AppError> {
        let state = app_for_retrieval.state::<AppState>();
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        retrieve_context(
            retrieval_client.as_ref(),
            &memory_target,
            &conn,
            &session_for_retrieval,
            &text_for_retrieval,
        )
    })
    .await
    .map_err(|e| AppError::Llm(format!("检索任务失败: {e}")))??;

    // ④ 组装请求（系统 Prompt + 摘要头 + 最近 N 条 + 本条）
    let request = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        build_chat_request(
            &conn,
            &session_id,
            &retrieved,
            (relation.0, relation.1.as_str()),
            &text,
        )?
    };

    // ⑤ 流式任务（不阻塞命令返回）
    let session = session_id.clone();
    let abort_flag = {
        let mut registry = abort_registry().lock().map_err(|e| AppError::Db(format!("{e}")))?;
        let flag = Arc::new(AtomicBool::new(false));
        registry.insert(session_id.clone(), flag.clone());
        flag
    };

    async_runtime::spawn(async move {
        run_chat_task(app, session, request, tiered, quota_embedded, abort_flag).await;
    });
    Ok(())
}

/// 中止指定会话（流循环在下一 chunk 检查标志后退出）。
#[tauri::command]
pub fn chat_abort(session_id: String) -> Result<(), AppError> {
    if let Some(flag) =
        abort_registry().lock().map_err(|e| AppError::Db(format!("{e}")))?.get(&session_id)
    {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

/// 对话任务主体（流式 emit → done → assistant 落库 → 蒸馏 spawn）。
async fn run_chat_task(
    app: AppHandle,
    session_id: String,
    request: ChatRequest,
    tiered: crate::db::models::TieredValue,
    quota_embedded: bool,
    abort_flag: Arc<AtomicBool>,
) {
    // 蒸馏输入的用户文本先提取（request 将被 chat_with_fallback 消费）
    let user_text = request
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);
    let (stream_result, chain_log) =
        chat_with_fallback(&client, &tiered, quota_embedded, request).await;

    for entry in &chain_log {
        // 回落链日志：只打档位与状态摘要，不打 Key/Prompt 全文
        eprintln!("[chat] {entry}");
    }

    let mut full_text = String::new();
    match stream_result {
        Ok(stream) => {
            let mut stream = std::pin::pin!(stream);
            while let Some(delta) = futures_util::StreamExt::next(&mut stream).await {
                if abort_flag.load(Ordering::Relaxed) {
                    // 中止：不落库完整回复（部分文本不作为完整回复）
                    let _ = app.emit(
                        "chat:stream:error",
                        StreamError {
                            session_id: session_id.clone(),
                            kind: "aborted".into(),
                            message: "已中止".into(),
                        },
                    );
                    return;
                }
                match delta {
                    Ok(d) => {
                        full_text.push_str(&d);
                        let _ = app.emit(
                            "chat:stream:delta",
                            StreamDelta { session_id: session_id.clone(), delta: d },
                        );
                    },
                    Err(e) => {
                        emit_error(&app, &session_id, &e);
                        return;
                    },
                }
            }
        },
        // 全档位失败：本地兜底话术（伪流：一条 delta）
        Err(phrase) => {
            full_text = phrase;
            let _ = app.emit(
                "chat:stream:delta",
                StreamDelta { session_id: session_id.clone(), delta: full_text.clone() },
            );
        },
    }

    if abort_flag.load(Ordering::Relaxed) {
        let _ = app.emit(
            "chat:stream:error",
            StreamError {
                session_id: session_id.clone(),
                kind: "aborted".into(),
                message: "已中止".into(),
            },
        );
        return;
    }

    // assistant 全文落库
    let db_result = {
        let state = app.state::<AppState>();
        let conn = state.db.lock();
        match conn {
            Ok(conn) => messages::insert(&conn, &session_id, 1, &full_text),
            Err(e) => Err(AppError::Db(format!("获取数据库连接失败: {e}"))),
        }
    };
    if let Err(e) = db_result {
        eprintln!("[chat] assistant 落库失败（不影响已输出）: {e}");
    }

    let _ = app.emit("chat:stream:done", StreamDone { session_id: session_id.clone() });

    // 蒸馏 spawn（不阻塞 done；memory 档失败静默——对话仍正常，计划 §七）
    let distill_input = crate::llm::distill::DistillInput {
        session_id: session_id.clone(),
        user_text,
        assistant_text: full_text,
    };
    async_runtime::spawn(async move {
        let distill_client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);
        let memory_target = resolve_memory_target(&tiered, quota_embedded);
        let result = async_runtime::spawn_blocking(
            move || -> Result<crate::llm::distill::DistillOutcome, AppError> {
                // 阻塞段：锁 DB；LLM 调用（async）经 Handle::block_on 桥接（蒸馏为后台低优任务）
                let state = app.state::<AppState>();
                let conn = state.db.lock().map_err(|e| AppError::Db(format!("{e}")))?;
                let rt = tokio::runtime::Handle::current();
                let out = rt.block_on(crate::llm::distill::run_distill(
                    &distill_client,
                    &memory_target,
                    &conn,
                    &distill_input,
                ))?;

                // C5.5 共鸣结算（失败静默，不影响对话；conn/app 在此闭包作用域内）
                if let Some(strength) = out.resonance
                    && let Err(e) = crate::affection::settle_signal(
                        &conn,
                        &app,
                        crate::affection::rules::AffectionSignal::Resonance(strength),
                    )
                {
                    eprintln!("[affection] 共鸣结算失败: {e}");
                }
                // C7.1 禁忌结算：severity → -10/-20/-30（affection:changed(angry) 由结算天然 emit）；
                // 情绪化短句经 proactive:show 展示（数字已剥离，红线：话术无数值）
                if let Some(hit) = &out.taboo {
                    let delta = crate::llm::distill::taboo_delta(hit.severity) as i8;
                    if let Err(e) = crate::affection::settle_signal(
                        &conn,
                        &app,
                        crate::affection::rules::AffectionSignal::Taboo(delta),
                    ) {
                        eprintln!("[affection] 禁忌结算失败: {e}");
                    }
                    let session_id =
                        format!("proactive-taboo-{}", chrono::Local::now().timestamp_millis());
                    let _ = app.emit(
                        "proactive:show",
                        crate::llm::ProactiveShow {
                            session_id,
                            text: hit.emotion_hint.clone(),
                            mood: "angry".into(),
                        },
                    );
                }
                // C5 深度对话：单轮回复字数 ≥ 阈值 → +5（每消息至多一次）
                if distill_input.assistant_text.chars().count()
                    >= crate::config::DEEP_CHAT_MIN_CHARS
                    && let Err(e) = crate::affection::settle_signal(
                        &conn,
                        &app,
                        crate::affection::rules::AffectionSignal::DeepChat,
                    )
                {
                    eprintln!("[affection] 深聊结算失败: {e}");
                }
                Ok(out)
            },
        )
        .await;
        match result {
            // 展平 spawn_blocking 的 JoinError 包装（Err 携带任务 panic/join 错误）
            Ok(Ok(outcome)) => {
                if outcome.facts_written > 0 {
                    eprintln!(
                        "[distill] 沉淀 {} 条事实（summarized={}）",
                        outcome.facts_written, outcome.summarized
                    );
                }
            },
            Ok(Err(e)) => {
                // 静默失败：重试逻辑在 run_distill 内部（一次）；此处只记录
                eprintln!("[distill] 沉淀失败（对话不受影响）: {e}");
            },
            Err(join_err) => {
                eprintln!("[distill] 蒸馏任务异常退出: {join_err}");
            },
        }
    });
}

/// 混合检索：embed(query) → KNN + facts 近因。
/// **阻塞函数**：由调用方 spawn_blocking 包裹。
fn retrieve_context(
    client: &dyn LlmClient,
    memory_target: &LlmTarget,
    conn: &rusqlite::Connection,
    session_id: &str,
    query: &str,
) -> Result<RetrievedContext, AppError> {
    // embed 失败不阻断对话：记忆段退化为仅 facts（降级检索）
    // async_block 返回 Result<F::Output, AppError>，而 embed 的 F::Output 本身
    // 也是 Result（双层）——匹配内层即可，任何一层失败都走降级
    let memory_hits = match async_block(client.embed(memory_target, vec![query.to_string()])) {
        Ok(Ok(vectors)) if vectors.len() == 1 => {
            vec_memory::knn(conn, &vec_memory::embedding_bytes(&vectors[0]), 5)?
                .into_iter()
                .map(|(_, text)| text)
                .collect()
        },
        _ => Vec::new(),
    };
    let _ = session_id; // 摘要头在 build_chat_request 内处理
    let facts_list = crate::db::repositories::facts::recent_contents(conn, 8)?;
    Ok(RetrievedContext { memory_hits, facts: facts_list })
}

/// 测试桥接：blocking 线程内执行 async（检索的 LLM 调用桥）。
fn async_block<F: std::future::Future>(fut: F) -> Result<F::Output, AppError> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Ok(handle.block_on(fut)),
        Err(e) => Err(AppError::Llm(format!("无异步运行时: {e}"))),
    }
}

/// 组装对话请求：System Prompt + 摘要头 + 最近 N 条 + 本条消息。
fn build_chat_request(
    conn: &rusqlite::Connection,
    session_id: &str,
    retrieved: &RetrievedContext,
    relation: (i64, &str),
    text: &str,
) -> Result<ChatRequest, AppError> {
    let view = RelationView { level: relation.0, pet_name: "喵喵" };
    // C6.6 傲娇：距上次互动 ≥3 天 → 注入傲娇基调（用户主动对话同样生效）
    let last_interaction =
        crate::db::repositories::pet_states::get(conn).ok().and_then(|p| p.last_interaction);
    let ctx = crate::llm::prompt::ConversationCtx {
        tsundere: crate::llm::prompt::is_tsundere(
            last_interaction.as_deref(),
            chrono::Local::now(),
        ),
    };
    let mut system = build_system_prompt(retrieved, &view, &ctx);

    // 摘要头（会话超阈值后由蒸馏生成）
    if let Some(summary) = crate::db::repositories::summaries::get(conn, session_id)? {
        system.push_str("\n【此前的对话摘要】\n");
        system.push_str(&summary);
    }

    let mut messages = vec![ChatMessage { role: "system".into(), content: system }];
    for m in crate::db::repositories::messages::recent(
        conn,
        session_id,
        crate::config::MAX_CONTEXT_MESSAGES as i64,
    )? {
        messages.push(ChatMessage {
            role: if m.role == 0 { "user".into() } else { "assistant".into() },
            content: m.content,
        });
    }
    messages.push(ChatMessage { role: "user".into(), content: text.to_string() });
    Ok(ChatRequest { messages, max_tokens: None })
}

/// memory 档目标（蒸馏与 embedding 共用；勘误 #8）——委托 router 单一真源。
fn resolve_memory_target(
    tiered: &crate::db::models::TieredValue,
    quota_embedded: bool,
) -> LlmTarget {
    crate::llm::router::memory_target(tiered, quota_embedded)
}

fn emit_error(app: &AppHandle, session_id: &str, e: &AppError) {
    let kind = match e {
        AppError::Db(_) => "db",
        AppError::Llm(_) => "llm",
        AppError::Window(_) => "window",
        AppError::Probe(_) => "probe",
        AppError::Io(_) => "io",
        AppError::Config(_) => "config",
    };
    let _ = app.emit(
        "chat:stream:error",
        StreamError {
            session_id: session_id.to_string(),
            kind: kind.into(),
            message: e.to_string(),
        },
    );
}

/// 命令层辅助：读 tiered 配置（带锁）。
fn app_settings_tiered(
    conn: &rusqlite::Connection,
) -> Result<crate::db::models::TieredValue, AppError> {
    crate::db::repositories::app_settings::get_tiered(conn)
}

/// 命令层辅助：读额度模式（带锁）。
fn app_settings_quota(conn: &rusqlite::Connection) -> Result<i64, AppError> {
    crate::db::repositories::app_settings::get_quota_mode(conn)
}

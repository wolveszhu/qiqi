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

use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::async_runtime;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::repositories::{
    custom_model_settings, default_model_settings, messages, search_settings, vec_memory,
};
use crate::error::AppError;
use crate::llm::client::{ChatMessage, ChatRequest, LlmClient, ReqwestLlmClient};
use crate::llm::prompt::{RelationView, RetrievedContext, build_system_prompt, format_web_results};
use crate::llm::router::{self, ModelChain};
use crate::llm::web_search::{self, SearchResult};
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

/// 联网搜索引用来源（随 `chat:stream:sources` 下发，前端气泡底部渲染）。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamSources {
    session_id: String,
    sources: Vec<ChatSource>,
}

/// 单条引用来源（标题 + 链接；前端按 URL 新窗口打开）。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatSource {
    title: String,
    url: String,
}

/// 拖入/附件负载（D1.5；与前端 FilePayload 对齐，向后兼容：缺省纯文本）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePayload {
    pub name: String,
    /// "text" | "image"（Rust 侧再次校验，前端兜底层）。
    pub kind: String,
    /// text=utf8 内容；image=base64 data URL（含 `data:...;base64,` 前缀）。
    pub content: String,
    pub mime: String,
}

/// 单文件大小上限（红线：base64 不写日志、内存可控）。
const MAX_TEXT_BYTES: usize = 1024 * 1024; // 1MB
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024; // 10MB
const MAX_DROPPED_FILES: usize = 5;

/// 发送对话（流式；命令立即返回，结果经事件推送）。
#[tauri::command]
pub async fn chat_send_stream(
    app: AppHandle,
    session_id: String,
    text: String,
    attachments: Vec<FilePayload>,
    web_search: bool,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);

    // 附件类型校验（kind 仅 text/image；非法 → Config 错误）
    for a in &attachments {
        if a.kind != "text" && a.kind != "image" {
            return Err(AppError::Config(format!("不支持的附件类型: {}", a.kind)));
        }
    }

    // ① 配置快照（每次实时读取——改配置即生效，冷眼三问③）+ 两个模型源 + 编排链
    // 背景蒸馏**复用这同一条链**（同源同序 + 完整 failover），不再单独派生模型。
    let (relation, chain) = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        let tiered = app_settings_tiered(&conn)?;
        let quota = app_settings_quota(&conn)?;
        let pet = crate::db::repositories::pet_states::get(&conn)?;
        let embedded = quota == crate::config::QUOTA_MODE_EMBEDDED;
        // 自定义（用户自配，优先）始终入链；内置仅在「内置额度」模式兜底
        let custom = custom_model_settings::list_enabled_llm(&conn)?;
        let builtin =
            if embedded { default_model_settings::list_available_llm(&conn)? } else { Vec::new() };
        let (cursor, last_ok) = router::next_builtin_slot();
        let chain = router::build_chain(&router::ChainInput {
            custom: &custom,
            builtin: &builtin,
            builtin_cursor: cursor,
            builtin_last_ok: last_ok,
            tiered: &tiered,
        });
        ((pet.relationship_level, pet.pet_name), chain)
    };

    // ② user 消息落库（图片不持久化内容，仅留标记；MVP 记忆不存图）
    {
        let mut stored = text.clone();
        if attachments.iter().any(|a| a.kind == "image") {
            stored.push_str("（附图片）");
        }
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        messages::insert(&conn, &session_id, 0, &stored)?;
    }

    // ③ 混合检索：查询向量先算（**不持 DB 锁**）。
    // 锁纪律（本次修复）：embed 是网络调用，向量端点失联时可达 30s 超时——原实现
    // 持锁 `block_on(embed)` 会把 DB 锁占满整段网络时间，而面板挂载触发的同步 IPC
    // 命令（`pet_get_state`/`settings_get_typed` 等）在 Tauri 主线程上等锁，
    // 整个应用事件循环随之冻结（表现即「等回复时切页 → 窗口未响应/假死」）。
    // embed 失败不阻断对话：query 向量置 None，记忆段退化为仅 facts（降级检索）。
    let embed_target = router::embed_target();
    let query_vector = client
        .embed(&embed_target, vec![text.clone()])
        .await
        .ok()
        .and_then(|vectors| vectors.into_iter().next());
    let session_for_retrieval = session_id.clone();
    let app_for_retrieval = app.clone();
    let retrieved = async_runtime::spawn_blocking(move || -> Result<RetrievedContext, AppError> {
        let state = app_for_retrieval.state::<AppState>();
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        // DB 段只做毫秒级 KNN + facts 近因（纯 DB，禁在此调 embed/LLM）
        retrieve_context(&conn, query_vector.as_deref(), &session_for_retrieval)
    })
    .await
    .map_err(|e| AppError::Llm(format!("检索任务失败: {e}")))??;

    // ③.5 联网搜索（应用侧检索增强；仅开关开启且已配置 Key 时发起）。
    // 失败/未配置/空结果 → 静默回落普通对话（不中断、不报错，应用不崩不卡）。
    let web_results: Option<Vec<SearchResult>> = if web_search {
        let settings = {
            let conn =
                state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
            search_settings::get(&conn)
        };
        match settings {
            Ok(s) if s.is_ready() => match web_search::web_search(&s, &text).await {
                Ok(r) if !r.is_empty() => {
                    // 下发引用来源（前端气泡底部渲染）；注入上下文在 build_chat_request 内追加
                    let _ = app.emit(
                        "chat:stream:sources",
                        StreamSources {
                            session_id: session_id.clone(),
                            sources: r
                                .iter()
                                .map(|x| ChatSource { title: x.title.clone(), url: x.url.clone() })
                                .collect(),
                        },
                    );
                    Some(r)
                },
                Ok(_) => None,
                Err(e) => {
                    eprintln!("[web_search] 失败，回落普通对话: {e}");
                    None
                },
            },
            _ => None,
        }
    } else {
        None
    };

    // ④ 组装请求（系统 Prompt + 摘要头 + 最近 N 条 + 本条）
    // E3.5：慢回复基调——读取上一轮响应耗时，超阈值则本轮回注轻量歉意（下一轮生效）
    let slow_reply = {
        let m = state
            .last_response_ms
            .lock()
            .map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?;
        m.map(|ms| ms > crate::llm::prompt::SLOW_REPLY_THRESHOLD_MS).unwrap_or(false)
    };
    let request = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        build_chat_request(
            &conn,
            &session_id,
            &retrieved,
            ChatBuildInput {
                relation: (relation.0, relation.1),
                text: &text,
                attachments: &attachments,
                slow_reply,
                web_results: web_results.as_deref(),
            },
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
        run_chat_task(app, session, request, RunChatParams { chain, abort_flag }).await;
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

/// 会话列表项 DTO（camelCase；与前端 `ChatSessionDto` 对齐）。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummaryDto {
    pub session_id: String,
    /// 标题预览：首条 user 消息（空 → UI 显示"新对话"）。
    pub preview: String,
    pub last_at: String,
    pub message_count: i64,
}

/// 会话消息 DTO（role：0=user / 1=assistant）。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDto {
    pub id: i64,
    pub role: i64,
    pub content: String,
}

/// 会话列表上限（UI 只列最近会话，防历史无限增长拖慢首屏）。
const SESSION_LIST_LIMIT: i64 = 50;
/// 单会话消息回传上限（超长会话只回最后 N 条，防序列化与渲染卡顿）。
const SESSION_MESSAGES_LIMIT: i64 = 200;

/// 会话列表（窗口 B「对话」Tab 左侧列表；按最后活跃倒序）。
#[tauri::command]
pub fn chat_list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionSummaryDto>, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    list_sessions_inner(&conn)
}

/// 读取指定会话的消息（时间正序，消息区回看用）。
#[tauri::command]
pub fn chat_get_messages(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MessageDto>, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    Ok(messages::full_by_session(&conn, &session_id, SESSION_MESSAGES_LIMIT)?
        .into_iter()
        .map(|m| MessageDto { id: m.id, role: m.role, content: m.content })
        .collect())
}

/// 删除会话（messages + summaries 同事务，理由见仓储文档）。
#[tauri::command]
pub fn chat_delete_session(session_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    messages::delete_session(&conn, &session_id)
}

/// 数据库锁错误统一映射（与既有命令层口径一致）。
fn db_lock_err<T: std::fmt::Display>(e: T) -> AppError {
    AppError::Db(format!("获取数据库连接失败: {e}"))
}

/// 会话列表 inner（薄壳 → inner，便于 `#[test]` 直调，F1.2）。
fn list_sessions_inner(conn: &rusqlite::Connection) -> Result<Vec<SessionSummaryDto>, AppError> {
    Ok(messages::list_sessions(conn, SESSION_LIST_LIMIT)?
        .into_iter()
        .map(|s| SessionSummaryDto {
            session_id: s.session_id,
            preview: s.preview,
            last_at: s.last_at,
            message_count: s.message_count,
        })
        .collect())
}

/// 解析拖入文件（D1.5）：文件读取全在 Rust（边界隔离，base64 不写日志）。
///
/// 文本 → utf8 内容；图片 → base64 data URL；大小/数量超限 → `AppError::Config`。
/// 仅返回结构化负载，后续随 `chat_send_stream` 的 `attachments` 发送。
#[tauri::command]
pub fn chat_resolve_dropped_files(paths: Vec<String>) -> Result<Vec<FilePayload>, AppError> {
    if paths.len() > MAX_DROPPED_FILES {
        return Err(AppError::Config(format!("一次最多拖入 {MAX_DROPPED_FILES} 个文件")));
    }
    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        // 仅在文件名层面判定类型（不读取内容即可分流，少做 IO）
        let kind = if is_image_path(&path) { "image" } else { "text" };
        let raw = std::fs::read(&path).map_err(|e| AppError::Io(format!("读取文件失败: {e}")))?;
        let bytes = raw.len();
        let (content, mime) = if kind == "image" {
            if bytes > MAX_IMAGE_BYTES {
                return Err(AppError::Config(format!(
                    "图片 {} 超过 {}MB 上限",
                    path,
                    MAX_IMAGE_BYTES / 1024 / 1024
                )));
            }
            let mime = detect_mime(&path);
            let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
            (format!("data:{mime};base64,{b64}"), mime)
        } else {
            if bytes > MAX_TEXT_BYTES {
                return Err(AppError::Config(format!(
                    "文本 {} 超过 {}MB 上限",
                    path,
                    MAX_TEXT_BYTES / 1024 / 1024
                )));
            }
            // 二进制文本（如程序）→ 视为不支持，拒绝而非乱码
            let text = String::from_utf8(raw.clone())
                .map_err(|_| AppError::Config(format!("文件 {} 不是可读取的文本/图片", path)))?;
            (text, "text/plain".into())
        };
        out.push(FilePayload {
            name: std::path::Path::new(&path)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone()),
            kind: kind.into(),
            content,
            mime,
        });
    }
    Ok(out)
}

/// 按扩展名粗判图片（D1.5：视觉模型要求；非图片一律走文本分支）。
fn is_image_path(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    p.ends_with(".png")
        || p.ends_with(".jpg")
        || p.ends_with(".jpeg")
        || p.ends_with(".gif")
        || p.ends_with(".webp")
        || p.ends_with(".bmp")
}

/// 图片 MIME 推测（数据与模型对齐用；非关键，缺省 png）。
fn detect_mime(path: &str) -> String {
    let p = path.to_ascii_lowercase();
    match p.rsplit('.').next() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        _ => "image/png",
    }
    .into()
}

/// 对话任务主体（流式 emit → done → assistant 落库 → 蒸馏 spawn）。
/// 流式任务的运行时依赖（打包避免参数列表超过 clippy `too_many_arguments` 上限）。
struct RunChatParams {
    /// 主对话与后台蒸馏**共用**的编排链（自定义 → 内置 → 三档）。
    chain: ModelChain,
    abort_flag: Arc<AtomicBool>,
}

async fn run_chat_task(
    app: AppHandle,
    session_id: String,
    request: ChatRequest,
    params: RunChatParams,
) {
    let RunChatParams { chain, abort_flag } = params;
    // E3.5：计时起点（首 token 前到 done 的时长，用于下一轮 slow_reply 判定）
    let started = std::time::Instant::now();
    // 蒸馏输入的用户文本先提取（request 将被 chat_with_fallback 消费）
    let user_text = request
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.as_text())
        .unwrap_or_default();

    let client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);
    // 额度记账兜底口径：请求全字符数（request 即将被链消费，先取）
    let prompt_chars = crate::llm::quota::request_chars(&request);
    let (stream_result, chain_log, hit) = router::chat_with_chain(&client, &chain, request).await;

    for entry in &chain_log {
        // 回落链日志：只打档位与状态摘要，不打 Key/Prompt 全文
        eprintln!("[chat] {entry}");
    }
    // 记账辅助：hit 转借用三元组（None = 链路全失败，不落库）
    let hit_ref = hit.as_ref().map(|h| (h.tier.as_str(), h.model.as_str(), h.platform.as_str()));
    // 记账失败不影响对话主流程（仅 log）
    let charge = |conn: &rusqlite::Connection,
                  usage: Option<crate::llm::client::TokenUsage>,
                  completion_chars: usize,
                  ok: bool,
                  error: Option<&str>| {
        let chars =
            crate::llm::quota::CallChars { prompt: prompt_chars, completion: completion_chars };
        if let Err(e) =
            crate::llm::quota::record_call(conn, "chat", hit_ref, usage, chars, ok, error)
        {
            eprintln!("[quota] 记账失败: {e}");
        }
    };

    let mut usage: Option<crate::llm::client::TokenUsage> = None;
    let mut full_text = String::new();
    match stream_result {
        Ok(stream) => {
            let mut stream = std::pin::pin!(stream);
            while let Some(event) = futures_util::StreamExt::next(&mut stream).await {
                if abort_flag.load(Ordering::Relaxed) {
                    // 中止：不落库完整回复（部分文本不作为完整回复）；记账记失败条目（0 点）
                    {
                        let state = app.state::<AppState>();
                        if let Ok(conn) = state.db.lock() {
                            charge(&conn, usage, full_text.chars().count(), false, Some("已中止"));
                        }
                    }
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
                match event {
                    Ok(crate::llm::client::StreamEvent::Delta(d)) => {
                        full_text.push_str(&d);
                        let _ = app.emit(
                            "chat:stream:delta",
                            StreamDelta { session_id: session_id.clone(), delta: d },
                        );
                    },
                    Ok(crate::llm::client::StreamEvent::Usage(u)) => usage = Some(u),
                    Err(e) => {
                        let err_str = e.to_string();
                        {
                            let state = app.state::<AppState>();
                            if let Ok(conn) = state.db.lock() {
                                charge(
                                    &conn,
                                    usage,
                                    full_text.chars().count(),
                                    false,
                                    Some(&err_str),
                                );
                            }
                        }
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

    // 成功记账（流正常收尾；hit 为 None = 全链失败走本地话术，不落库）
    if hit.is_some() {
        let state = app.state::<AppState>();
        if let Ok(conn) = state.db.lock() {
            charge(&conn, usage, full_text.chars().count(), true, None);
        }
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

    // E3.5：记录本轮响应耗时（毫秒），供下一轮 slow_reply 判定（内存态，重启清零）
    {
        let state = app.state::<AppState>();
        if let Ok(mut m) = state.last_response_ms.lock() {
            *m = Some(started.elapsed().as_millis() as u64);
        }
    }

    // C5 深度对话：单轮回复字数 ≥ 阈值 → +5（每消息至多一次）。
    //
    // **必须独立于蒸馏结算**：深聊判定只看回复长度、不依赖 LLM；原先挂在蒸馏闭包里
    // `run_distill(...)?` 之后，蒸馏一失败（`?` 短路）本轮应得的好感度就被一起吞掉
    // ——表现为"聊了天却没有任何流水"。
    let deep_chat = full_text.chars().count() >= crate::config::DEEP_CHAT_MIN_CHARS;
    if deep_chat {
        let settled = {
            let state = app.state::<AppState>();
            match state.db.lock() {
                Ok(conn) => crate::affection::settle_signal(
                    &conn,
                    &app,
                    crate::affection::rules::AffectionSignal::DeepChat,
                ),
                Err(e) => Err(AppError::Db(format!("获取数据库连接失败: {e}"))),
            }
        };
        if let Err(e) = settled {
            eprintln!("[affection] 深聊结算失败: {e}");
        }
    }

    // 蒸馏 spawn（不阻塞 done；失败静默——对话仍正常，计划 §七）
    let distill_input = crate::llm::distill::DistillInput {
        session_id: session_id.clone(),
        user_text,
        assistant_text: full_text,
    };
    async_runtime::spawn(async move {
        let distill_client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);
        // 蒸馏**复用主对话链**：自定义始终入链 + 内置额度开时兜底 + 三档尾部，完整 failover。
        // 向量写入走独立的 embed 端点（失败仅降级，不阻断沉淀）
        let embed_target = router::embed_target();
        // 链为空（自定义/内置/三档全未配置）→ 跳过（不盲目请求、不报错）
        if chain.targets.is_empty() {
            log_distill_skipped_once();
            return;
        }
        let result = async_runtime::spawn_blocking(
            move || -> Result<crate::llm::distill::DistillOutcome, AppError> {
                let state = app.state::<AppState>();
                // 锁纪律（本次修复）：蒸馏全程含多次 LLM/embed 网络调用（每段可达 30s
                // 超时），**严禁持锁跨任何网络调用**——同步 IPC 命令在主线程等锁，
                // 锁被占住即整窗冻结。DB 段全部改为「取锁-用-释放」的毫秒级短段。
                let pet_name = {
                    let conn = state.db.lock().map_err(|e| AppError::Db(format!("{e}")))?;
                    crate::db::repositories::pet_states::get(&conn)?.pet_name
                };
                let rt = tokio::runtime::Handle::current();
                let out = rt.block_on(crate::llm::distill::run_distill(
                    &distill_client,
                    &chain,
                    &embed_target,
                    &state.db,
                    &distill_input,
                    &pet_name,
                ))?;

                // C5.5 共鸣结算（失败静默，不影响对话；短锁段）
                if let Some(strength) = out.resonance {
                    let settled = (|| {
                        let conn = state.db.lock().map_err(|e| AppError::Db(format!("{e}")))?;
                        crate::affection::settle_signal(
                            &conn,
                            &app,
                            crate::affection::rules::AffectionSignal::Resonance(strength),
                        )
                    })();
                    if let Err(e) = settled {
                        eprintln!("[affection] 共鸣结算失败: {e}");
                    }
                }
                // C7.1 禁忌结算：severity → -10/-20/-30（affection:changed(angry) 由结算天然 emit）；
                // 情绪化短句经 proactive:show 展示（数字已剥离，红线：话术无数值）
                if let Some(hit) = &out.taboo {
                    let delta = crate::llm::distill::taboo_delta(hit.severity) as i8;
                    let settled = (|| {
                        let conn = state.db.lock().map_err(|e| AppError::Db(format!("{e}")))?;
                        crate::affection::settle_signal(
                            &conn,
                            &app,
                            crate::affection::rules::AffectionSignal::Taboo(delta),
                        )
                    })();
                    if let Err(e) = settled {
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
                // 深聊结算已移至本函数蒸馏**之前**（不依赖蒸馏成败），此处不再重复结算。
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

/// 混合检索 DB 段：KNN（查询向量可选）+ facts 近因。
/// **纯 DB、毫秒级**：由调用方持锁调用；查询向量的网络 embed 已上移至
/// `chat_send_stream`（锁外）——本函数禁止调用 LLM/embed（锁跨网络调用会冻结整窗）。
fn retrieve_context(
    conn: &rusqlite::Connection,
    query_vector: Option<&[f32]>,
    session_id: &str,
) -> Result<RetrievedContext, AppError> {
    let memory_hits = match query_vector {
        Some(v) => vec_memory::knn(conn, &vec_memory::embedding_bytes(v), 5)?
            .into_iter()
            .map(|(_, text)| text)
            .collect(),
        None => Vec::new(),
    };
    let _ = session_id; // 摘要头在 build_chat_request 内处理
    let facts_list = crate::db::repositories::facts::recent_contents(conn, 8)?;
    Ok(RetrievedContext { memory_hits, facts: facts_list })
}

/// `build_chat_request` 入参（打包避免 `too_many_arguments`；生命周期随请求组装）。
struct ChatBuildInput<'a> {
    relation: (i64, String),
    text: &'a str,
    attachments: &'a [FilePayload],
    slow_reply: bool,
    web_results: Option<&'a [SearchResult]>,
}

/// 组装对话请求：System Prompt + 摘要头 + 最近 N 条 + 本条消息（含多模态附件）。
///
/// `web_results`：联网检索到的资料（应用侧检索增强）；非空时追加到 system 上下文段。
fn build_chat_request(
    conn: &rusqlite::Connection,
    session_id: &str,
    retrieved: &RetrievedContext,
    input: ChatBuildInput<'_>,
) -> Result<ChatRequest, AppError> {
    let view = RelationView { level: input.relation.0, pet_name: input.relation.1 };
    // C6.6 傲娇：距上次互动 ≥3 天 → 注入傲娇基调（用户主动对话同样生效）
    let last_interaction =
        crate::db::repositories::pet_states::get(conn).ok().and_then(|p| p.last_interaction);
    let ctx = crate::llm::prompt::ConversationCtx {
        tsundere: crate::llm::prompt::is_tsundere(
            last_interaction.as_deref(),
            chrono::Local::now(),
        ),
        slow_reply: input.slow_reply,
    };
    let mut system = build_system_prompt(retrieved, &view, &ctx);
    // 联网检索资料段（空则无追加；纯函数，见 prompt::format_web_results）
    system.push_str(&format_web_results(input.web_results.unwrap_or(&[])));

    // 摘要头（会话超阈值后由蒸馏生成）
    if let Some(summary) = crate::db::repositories::summaries::get(conn, session_id)? {
        system.push_str("\n【此前的对话摘要】\n");
        system.push_str(&summary);
    }

    let mut messages = vec![ChatMessage { role: "system".into(), content: system.into() }];
    for m in crate::db::repositories::messages::recent(
        conn,
        session_id,
        crate::config::MAX_CONTEXT_MESSAGES as i64,
    )? {
        messages.push(ChatMessage {
            role: if m.role == 0 { "user".into() } else { "assistant".into() },
            content: m.content.into(),
        });
    }
    // 本条消息：有附件 → vision content 数组（文本 part + 图片 part）；否则纯文本
    let user_content = if input.attachments.is_empty() {
        crate::llm::client::MessageContent::Text(input.text.to_string())
    } else {
        let mut parts =
            vec![crate::llm::client::ContentPart::Text { text: input.text.to_string() }];
        for a in input.attachments {
            if a.kind == "image" {
                parts.push(crate::llm::client::ContentPart::ImageUrl {
                    image_url: crate::llm::client::ImageUrl { url: a.content.clone() },
                });
            } else {
                // 文本附件：内容拼入文本 part（便于模型引用）
                parts.push(crate::llm::client::ContentPart::Text {
                    text: format!("[附件 {}]\n{}", a.name, a.content),
                });
            }
        }
        crate::llm::client::MessageContent::Parts(parts)
    };
    messages.push(ChatMessage { role: "user".into(), content: user_content });
    Ok(ChatRequest { messages, max_tokens: None })
}

/// 沉淀跳过提示（进程内只打一次，避免每轮对话刷屏）：
/// 编排链为空（自定义/内置/三档全未配置）→ 没有可用模型，蒸馏静默跳过。
fn log_distill_skipped_once() {
    use std::sync::OnceLock;
    static WARNED: OnceLock<bool> = OnceLock::new();
    if WARNED.get().is_none() {
        eprintln!("[distill] 编排链为空（自定义/内置/三档均未配置）；沉淀已跳过");
        let _ = WARNED.set(true);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn list_sessions_inner_on_empty_db_is_empty() {
        let (_dir, conn) = fresh_db();
        assert!(list_sessions_inner(&conn).expect("列会话").is_empty(), "空库返回空列表");
    }

    #[test]
    fn list_sessions_inner_maps_dto() {
        let (_dir, conn) = fresh_db();
        messages::insert(&conn, "s1", 0, "你好呀").expect("插入");
        messages::insert(&conn, "s1", 1, "我在呢").expect("插入");

        let list = list_sessions_inner(&conn).expect("列会话");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].session_id, "s1");
        assert_eq!(list[0].preview, "你好呀", "preview 取 user 首条");
        assert_eq!(list[0].message_count, 2);
        assert!(!list[0].last_at.is_empty());
    }
}

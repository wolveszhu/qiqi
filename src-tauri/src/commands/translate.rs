//! 划词翻译命令薄层（参数校验 + 转发领域模块）。
//!
//! 事件 `translate:result` / `translate:error` 由命令层 emit（与 chat 组现码同风格）；
//! 错误一律 `AppError` 传播，前端按 `kind` 用 `t()` 本地化兜底，DTO 不含任何明文 Key。

use rusqlite::Connection;
use tauri::{AppHandle, Emitter, State};

use crate::db::models::{
    BuiltinTranslateProviderDto, CapturedSelectionDto, CustomTranslateProviderDto,
    SaveCustomTranslateProviderPayload, SaveTranslateSettingsPayload, TranslateErrorPayload,
    TranslateResultDto, TranslateSettingsDto,
};
use crate::db::repositories::{
    builtin_translate_providers as builtin_repo, custom_translate_providers as custom_repo,
    quota_usage,
};
use crate::error::AppError;
use crate::probe::selection;
use crate::state::AppState;
use crate::translate::service;

fn db_lock_err<T: std::fmt::Display>(e: T) -> AppError {
    AppError::Db(format!("获取数据库连接失败: {e}"))
}

/// AppError 的 kind 字符串（与前端 `AppErrorKind` 小写枚举一致）。
fn error_kind(e: &AppError) -> &'static str {
    match e {
        AppError::Db(_) => "db",
        AppError::Llm(_) => "llm",
        AppError::Window(_) => "window",
        AppError::Probe(_) => "probe",
        AppError::Io(_) => "io",
        AppError::Config(_) => "config",
    }
}

/// 读取划词翻译配置（Key 脱敏回传）。
#[tauri::command]
pub fn translate_settings_get(
    state: State<'_, AppState>,
) -> Result<TranslateSettingsDto, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    service::get_settings(&conn)
}

/// 保存划词翻译配置（Key 缺省 = 不改动；空串 = 清空）。
#[tauri::command]
pub fn translate_settings_save(
    payload: SaveTranslateSettingsPayload,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    service::save_settings(&conn, payload)
}

/// 用户点击小图标后取词（UIA 待接入；当前为剪贴板法）。
/// 工具条显示期间直接复用手势探测留在剪贴板里的选中文本，不再重复注入 `Ctrl+C`。
#[tauri::command]
pub fn translate_capture_selection(
    selection_key: String,
) -> Result<CapturedSelectionDto, AppError> {
    selection::capture_selection(&selection_key)
}

/// 复制译文到系统剪贴板（用户主动点击，Rust 侧写入）。
#[tauri::command]
pub fn translate_copy_result(text: String) -> Result<(), AppError> {
    selection::write_clipboard_text(&text)
}

/// 显示译文浮窗（选区旁定位；Rust 侧完成 DPI / 多显示器夹取）。
///
/// `compact=false` 表示胶囊工具条已切换为译文卡片（工具条消失）：此刻取词已完成，
/// 结束手势捕获状态；剪贴板里的选中词保留不动（用户可能只是想复制它）。
/// `compact=true` 的首次弹出不清理。
#[tauri::command]
pub fn translate_popup_show(x: i32, y: i32, compact: bool, app: AppHandle) -> Result<(), AppError> {
    if !compact {
        crate::probe::selection::clear_gesture_clipboard();
    }
    crate::window::manager::show_translator(&app, x, y, compact)
}

/// 选区 / 鼠标移动时重定位浮窗（不改变可见性）。
#[tauri::command]
pub fn translate_popup_position(
    x: i32,
    y: i32,
    compact: bool,
    app: AppHandle,
) -> Result<(), AppError> {
    crate::window::manager::move_translator(&app, x, y, compact)
}

/// 隐藏浮窗（Esc / 点击外部 / 关闭按钮；窗口不销毁）。
#[tauri::command]
pub fn translate_popup_hide(app: AppHandle) -> Result<(), AppError> {
    crate::window::manager::hide_translator(&app)
}

/// 翻译选中文本；成功后 emit `translate:result`，失败 emit `translate:error` 并返回 Err。
#[tauri::command]
pub async fn translate_execute(
    text: String,
    target_lang: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TranslateResultDto, AppError> {
    match service::execute(&state, &text, target_lang.as_deref()).await {
        Ok(dto) => {
            let _ = app.emit("translate:result", &dto);
            Ok(dto)
        },
        Err(e) => {
            let _ = app.emit(
                "translate:error",
                TranslateErrorPayload {
                    result_id: String::new(),
                    kind: error_kind(&e).to_string(),
                    retryable: true,
                },
            );
            Err(e)
        },
    }
}

/// 重新翻译（同原文 + 当前 provider）；每次真实请求各记一条内置额度。
#[tauri::command]
pub async fn translate_retry(
    result_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TranslateResultDto, AppError> {
    match service::retry(&state, &result_id).await {
        Ok(dto) => {
            let _ = app.emit("translate:result", &dto);
            Ok(dto)
        },
        Err(e) => {
            let _ = app.emit(
                "translate:error",
                TranslateErrorPayload {
                    result_id,
                    kind: error_kind(&e).to_string(),
                    retryable: true,
                },
            );
            Err(e)
        },
    }
}

// ---------- 用户自填多服务商（按 sort_order 优先，内置兜底） ----------

fn custom_key_fingerprint(key: &str) -> Option<String> {
    if key.len() >= 4 { Some(key[key.len() - 4..].to_string()) } else { None }
}

/// inner：组装脱敏 DTO（Key / APPID 明文绝不出现在返回结构里）。
fn list_custom_inner(conn: &Connection) -> Result<Vec<CustomTranslateProviderDto>, AppError> {
    Ok(custom_repo::list_all(conn)?
        .into_iter()
        .map(|r| {
            // 先取借用型字段，再移动 String 字段（避免部分移动后借用 row）。
            let fingerprint = custom_key_fingerprint(&r.api_key);
            let has_key = !r.api_key.is_empty();
            let has_app_id = !r.app_id.is_empty();
            let enabled = r.is_enabled();
            CustomTranslateProviderDto {
                id: r.id,
                name: r.name,
                provider: r.provider,
                has_app_id,
                has_key,
                key_fingerprint: fingerprint,
                enabled,
            }
        })
        .collect())
}

/// inner：新增 / 编辑；Key 缺省不改、空串清空，`enabled` 缺省沿用原值（新建默认启用）。
fn save_custom_inner(
    conn: &Connection,
    payload: SaveCustomTranslateProviderPayload,
) -> Result<i64, AppError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::Config("服务商名称不能为空".into()));
    }
    // 讯飞是内置专属通道（需三项凭据），自填表只有两列，显式拒绝以免产生不可用配置。
    if payload.provider == "xfyun" {
        return Err(AppError::Config("讯飞翻译为内置服务，无需自填凭据".into()));
    }
    let _ = crate::translate::provider::provider_for(&payload.provider)?;
    // APPID 缺省 = 不改动存量（编辑时前端只拿到 hasAppId，拿不到明文）。
    let app_id = match payload.app_id {
        Some(value) => value,
        None => match payload.id {
            Some(id) => custom_repo::get(conn, id)?.map(|r| r.app_id).unwrap_or_default(),
            None => String::new(),
        },
    };
    match payload.id {
        None => custom_repo::insert(
            conn,
            name,
            &payload.provider,
            &app_id,
            payload.api_key.as_deref().unwrap_or(""),
        ),
        Some(id) => {
            let is_can_use = match payload.enabled {
                Some(true) => 1,
                Some(false) => 2,
                None => custom_repo::get(conn, id)?.map(|r| r.is_can_use).unwrap_or(1),
            };
            custom_repo::update(conn, id, name, &payload.provider, &app_id, is_can_use)?;
            if let Some(key) = payload.api_key.as_deref() {
                custom_repo::update_api_key(conn, id, key)?;
            }
            Ok(id)
        },
    }
}

/// 读取用户自填的多个翻译服务商（Key 脱敏回传）。
#[tauri::command]
pub fn translate_list_custom_providers(
    state: State<'_, AppState>,
) -> Result<Vec<CustomTranslateProviderDto>, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    list_custom_inner(&conn)
}

/// 新增 / 编辑自填服务商，返回行 id。
#[tauri::command]
pub fn translate_save_custom_provider(
    payload: SaveCustomTranslateProviderPayload,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    save_custom_inner(&conn, payload)
}

/// 删除自填服务商。
#[tauri::command]
pub fn translate_delete_custom_provider(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    custom_repo::delete(&conn, id)
}

/// 上移 / 下移自填服务商（列表顺序即优先级）。
#[tauri::command]
pub fn translate_move_custom_provider(
    id: i64,
    up: bool,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    custom_repo::move_by_one(&conn, id, up).map(|_| ())
}

// ---------- 内置服务商（百度 + 讯飞）：只读用量视图（供错误诊断） ----------

/// inner：组装脱敏视图；用量取当月按平台汇总，凭据只回 `has` + 末 4 位指纹。
fn list_builtin_inner(conn: &Connection) -> Result<Vec<BuiltinTranslateProviderDto>, AppError> {
    let period = chrono::Local::now().format("%Y-%m").to_string();
    let mut out = Vec::new();
    for row in builtin_repo::list_all(conn)? {
        let (used, requests) = quota_usage::platform_used(conn, &period, &row.name)?;
        // 先取借用型字段，再移动 String（避免部分移动后借用 row）。
        let has_credential = row.has_credentials();
        let credential_fingerprint = row.credential_fingerprint();
        out.push(BuiltinTranslateProviderDto {
            id: row.id,
            provider_id: row.provider_id,
            name: row.name,
            enabled: row.enabled,
            monthly_granted: row.monthly_granted,
            used,
            requests,
            has_credential,
            credential_fingerprint,
        });
    }
    Ok(out)
}

/// 读取两家内置服务商的当月用量（凭据脱敏；只读，不暴露启停 / 上限写入口）。
#[tauri::command]
pub fn translate_list_builtin_providers(
    state: State<'_, AppState>,
) -> Result<Vec<BuiltinTranslateProviderDto>, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    list_builtin_inner(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    fn payload(
        name: &str,
        provider: &str,
        key: Option<&str>,
    ) -> SaveCustomTranslateProviderPayload {
        SaveCustomTranslateProviderPayload {
            id: None,
            name: name.into(),
            provider: provider.into(),
            app_id: None,
            api_key: key.map(str::to_string),
            enabled: Some(true),
        }
    }

    #[test]
    fn list_custom_never_leaks_plain_key() {
        let (_dir, conn) = fresh_db();
        custom_repo::insert(&conn, "我的 DeepL", "deepl", "", "sk-secret-4321").expect("新增");
        let list = list_custom_inner(&conn).expect("列表");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].key_fingerprint.as_deref(), Some("4321"));
        let json = serde_json::to_string(&list).expect("序列化");
        assert!(!json.contains("sk-secret"), "DTO 不得含明文 Key");
    }

    #[test]
    fn save_custom_trims_name_and_keeps_key_on_update() {
        let (_dir, conn) = fresh_db();
        let id =
            save_custom_inner(&conn, payload("  A  ", "deepl", Some("key-abcd"))).expect("新建");
        assert_eq!(list_custom_inner(&conn).expect("列表")[0].name, "A");

        let update = SaveCustomTranslateProviderPayload {
            id: Some(id),
            name: "B".into(),
            provider: "google".into(),
            app_id: None,
            api_key: None,
            enabled: Some(false),
        };
        save_custom_inner(&conn, update).expect("更新");
        let list = list_custom_inner(&conn).expect("列表");
        assert_eq!(list[0].name, "B");
        assert_eq!(list[0].provider, "google");
        assert!(list[0].has_key, "api_key 缺省不应清空存量 Key");
        assert!(!list[0].enabled);
    }

    #[test]
    fn save_custom_rejects_blank_name_and_unknown_provider() {
        let (_dir, conn) = fresh_db();
        assert!(save_custom_inner(&conn, payload("  ", "deepl", None)).is_err());
        assert!(save_custom_inner(&conn, payload("A", "bing", None)).is_err());
        assert!(
            save_custom_inner(&conn, payload("我的讯飞", "xfyun", Some("k"))).is_err(),
            "讯飞为内置专属，不得自填"
        );
    }

    #[test]
    fn list_builtin_never_leaks_plain_credentials() {
        let (_dir, conn) = fresh_db();
        let list = list_builtin_inner(&conn).expect("列表");
        assert_eq!(list.len(), 2, "百度 + 讯飞");
        assert!(list.iter().all(|p| p.has_credential));
        assert!(list.iter().all(|p| p.used == 0), "干净库当月用量为 0");
        let json = serde_json::to_string(&list).expect("序列化");
        for secret in [
            "20261107002714302",
            "YjDAPNYWWfN7STvnQjky",
            "c9955e0e",
            "ZmFjNGJkNDE1M2RlYjk1ZjIwYmVkNDg4",
            "b8cbf3e6d624f59e38f39b6b321b65f8",
        ] {
            assert!(!json.contains(secret), "DTO 不得含明文凭据");
        }
    }
}

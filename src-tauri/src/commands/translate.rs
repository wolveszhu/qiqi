//! 划词翻译命令薄层（参数校验 + 转发领域模块）。
//!
//! 事件 `translate:result` / `translate:error` 由命令层 emit（与 chat 组现码同风格）；
//! 错误一律 `AppError` 传播，前端按 `kind` 用 `t()` 本地化兜底，DTO 不含任何明文 Key。

use tauri::{AppHandle, Emitter, State};

use crate::db::models::{
    CapturedSelectionDto, SaveTranslateSettingsPayload, TranslateErrorPayload, TranslateResultDto,
    TranslateSettingsDto,
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

/// 用户点击小图标后取词（UIA 待接入；当前为剪贴板法，备份后恢复）。
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
#[tauri::command]
pub fn translate_popup_show(x: i32, y: i32, app: AppHandle) -> Result<(), AppError> {
    crate::window::manager::show_translator(&app, x, y)
}

/// 选区 / 鼠标移动时重定位浮窗（不改变可见性）。
#[tauri::command]
pub fn translate_popup_position(x: i32, y: i32, app: AppHandle) -> Result<(), AppError> {
    crate::window::manager::move_translator(&app, x, y)
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

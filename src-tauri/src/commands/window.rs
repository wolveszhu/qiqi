//! window 命令域（B1.4 临时通道；完整命令组归 C2）。
//!
//! B1.4 决策（登记）：设置窗 ✕ 的显隐经 **Rust 命令** `window_hide` 而非前端
//! JS API——避免为 B1 提前引入 capabilities 权限（B2 范围）；C2/B5 落
//! `src/api/window.ts` 与完整 `window_*` 命令组时替换本临时通道。

use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::window::manager;

/// 隐藏目标窗口（B1.4 临时显隐通道；C2 纳入 window_* 完整组）。
#[tauri::command]
pub fn window_hide(target: String, app: AppHandle) -> Result<(), AppError> {
    // 入参校验：target 必须是已声明的窗口 label（常量真源，防魔法字符串）。
    if target != manager::LABEL_MAIN && target != manager::LABEL_SETTINGS {
        return Err(AppError::Config(format!("未知窗口 label: {target}")));
    }
    let window = app
        .get_webview_window(&target)
        .ok_or_else(|| AppError::Window(format!("窗口不存在: {target}")))?;
    manager::hide_webview_window(&window)
}

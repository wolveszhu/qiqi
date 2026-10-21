//! 窗口域：label 常量 + 关闭=hide 拦截策略 + 显隐辅助（B1）。
//!
//! 边界（规范 §4.1）：窗口系统调用只在本域；命令薄层在 `commands/window.rs`；
//! TODO(C2): `drag.rs`（系统接管拖拽）、`click_through.rs`（穿透状态机）、
//! `tray.rs`、`shortcut.rs` 落位后由本模块统一出口。

use tauri::Window;

use crate::error::AppError;

/// 窗口 label 常量——**前后端契约**：`tauri.conf.json`、`src/main.ts` 分发、
/// Rust 侧 `get_webview_window` 三方一致；禁止散落魔法字符串（冷眼三问）。
pub const LABEL_MAIN: &str = "main";
pub const LABEL_SETTINGS: &str = "settings";

/// 隐藏窗口（关闭=hide 策略的核心动作；错误映射 `AppError::Window`）。
/// 参数为 `&Window`：`on_window_event` 传入的即 `&Window`。
pub fn hide_window(window: &Window) -> Result<(), AppError> {
    window.hide().map_err(|e| AppError::Window(e.to_string()))
}

// 关闭拦截策略（B1.2 决策，接线在 `lib.rs` 的 `on_window_event`）：
//
// - A/B 均拦截（`prevent_close` + `hide`）：桌宠是常驻应用，窗口 A 在
//   点击穿透（C2）实现前整窗可点，误触关闭不应退出进程；
// - 退出统一走托盘（TODO(C2.6)：Rust 侧 `app.exit`，绕过事件拦截）；
// - 开发期退出：任务管理器结束进程，或 C2 托盘到位后的"退出"菜单项。
//
// TODO(C2): 托盘"显示/设置"菜单落地时补 `show_window`。

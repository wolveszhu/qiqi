//! 系统接管拖拽（C2.5，rule.md §5 红线：拖拽一律 OS 实现，前端禁模拟）。
//!
//! 链路：前端 `pointerdown` → `window_start_drag` 命令 → `start_dragging()`；
//! 移动、跨屏、DPI 换算均由系统完成；前端只做形变反馈（`playAction("drag")`）。

use tauri::{AppHandle, Manager, WebviewWindow};

use super::manager::LABEL_MAIN;
use crate::error::AppError;

/// 开始拖拽窗口 A（由系统接管，同步返回）。
pub fn start_drag_main(app: &AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window(LABEL_MAIN)
        .ok_or_else(|| AppError::Window(format!("窗口不存在: {LABEL_MAIN}")))?;
    start_drag(&window)
}

/// 拖拽指定窗口（内部函数，便于测试与扩展）。
pub fn start_drag(window: &WebviewWindow) -> Result<(), AppError> {
    window.start_dragging().map_err(AppError::from)
}

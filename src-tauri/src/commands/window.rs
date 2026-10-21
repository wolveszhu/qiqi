//! window 命令域（薄层：参数校验 + 转发 window/manager 与 drag/click_through）。
//!
//! 纪律：命令只做转发与最小校验；窗口系统调用全部在 `window/` 域内（规范 §4.1）。

use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::state::{AppState, HitRegion};
use crate::window;

/// 开始拖拽窗口 A（系统接管；rule.md §5：前端禁模拟拖拽）。
#[tauri::command]
pub fn window_start_drag(app: AppHandle) -> Result<(), AppError> {
    window::drag::start_drag_main(&app)
}

/// 显隐窗口 A/B（关闭=hide 由 lib.rs 的 CloseRequested 拦截，见 B1.2）。
#[tauri::command]
pub fn window_show(target: String, app: AppHandle) -> Result<(), AppError> {
    let window = resolve(&app, &target)?;
    window.show().map_err(AppError::from)
}

/// 隐藏目标窗口（B1 临时通道转正：原 `window_hide` 语义并入本命令）。
#[tauri::command]
pub fn window_hide(target: String, app: AppHandle) -> Result<(), AppError> {
    let window = resolve(&app, &target)?;
    window.hide().map_err(AppError::from)
}

/// 直接设置穿透状态（C2.4 兜底通道；正常路径由 Rust 状态机自动翻转）。
#[tauri::command]
pub fn window_set_ignore_cursor_events(ignore: bool, app: AppHandle) -> Result<(), AppError> {
    let window = resolve(&app, window::manager::LABEL_MAIN)?;
    window.set_ignore_cursor_events(ignore).map_err(AppError::from)
}

/// 上报命中区域（窗口局部坐标；C2.2：前端低频、变化才调）。
#[tauri::command]
pub fn window_set_hit_region(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut slot =
        state.hit_region.lock().map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?;
    *slot = HitRegion { x, y, width, height };
    Ok(())
}

/// 穿透总闸（D1 输入框可见 / D4 边缘休眠时关闭 → 整窗接收点击）。
#[tauri::command]
pub fn window_set_click_through_enabled(
    enabled: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut slot = state
        .click_through_enabled
        .lock()
        .map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?;
    *slot = enabled;
    // 关总闸时立即解除穿透（不必等下一轮询）
    if !enabled {
        drop(slot);
        if let Some(window) = app.get_webview_window(window::manager::LABEL_MAIN) {
            window.set_ignore_cursor_events(false).map_err(AppError::from)?;
        }
    }
    Ok(())
}

/// 解析目标窗口（label 常量校验，禁魔法字符串）。
fn resolve<R: tauri::Runtime>(
    app: &AppHandle<R>,
    target: &str,
) -> Result<tauri::WebviewWindow<R>, AppError> {
    if target != window::manager::LABEL_MAIN && target != window::manager::LABEL_SETTINGS {
        return Err(AppError::Config(format!("未知窗口 label: {target}")));
    }
    app.get_webview_window(target).ok_or_else(|| AppError::Window(format!("窗口不存在: {target}")))
}

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

/// 上报命中区域（C2.2：前端低频、变化才调）。
///
/// 前端上报为**逻辑像素**（PIXI `getBounds()`，autoDensity 下即 CSS 像素）；
/// 穿透判定的另两个输入——cursor（`GetCursorPos`）与窗口位置（`outer_position()`）——
/// 均为**物理像素**，故此处单点换算 `× scale_factor` 后落缓存（窗口 A 命中坐标系
/// 的唯一换算出口，修复 DPI≠100% 时命中区错位导致整窗误穿透、无法拖动/点击）。
#[tauri::command]
pub fn window_set_hit_region(
    window: tauri::WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let scale = window.scale_factor().map_err(AppError::from)?;
    let mut slot =
        state.hit_region.lock().map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?;
    *slot = HitRegion {
        x: (x * scale).round() as i32,
        y: (y * scale).round() as i32,
        width: (width * scale).round() as i32,
        height: (height * scale).round() as i32,
    };
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

/// 气泡/输入框展开态：窗口 A 临时放大（保持形象中心不动），收起回常态贴合尺寸。
#[tauri::command]
pub fn window_set_bubble_mode(expanded: bool, app: AppHandle) -> Result<(), AppError> {
    window::manager::set_bubble_mode(&app, expanded)
}

/// 边缘吸附休眠态广播（D4.4）：前端进入/退出休眠时调用。
///
/// 作用：Rust 后台任务（日记生成等）据此静默；**内存态不落库**（重启即醒，
/// 休眠外观由前端 sleepSnap 恢复，开关状态另存 app_settings type=3）。
#[tauri::command]
pub fn window_set_sleep_mode(enabled: bool, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut flag =
        state.sleep_mode.lock().map_err(|e| AppError::Db(format!("获取休眠态锁失败: {e}")))?;
    *flag = enabled;
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

/// 窗口 A 外位置 + 当前显示器逻辑矩形（D4.4 贴边判定数据源）。
#[tauri::command]
pub fn window_get_outer_position(
    app: AppHandle,
) -> Result<crate::window::manager::OuterPositionView, AppError> {
    window::manager::outer_position_view(&app)
}

/// 贴边吸附休眠（edge ∈ top/left/right）：Rust 经 current_monitor 计算边界 + 半透明。
#[tauri::command]
pub fn window_snap_to_edge(edge: String, app: AppHandle) -> Result<(), AppError> {
    window::manager::snap_to_edge(&app, &edge)
}

/// 解除吸附（恢复透明度 1.0；位置由用户拖动决定，不自动回中）。
#[tauri::command]
pub fn window_restore_from_snap(app: AppHandle) -> Result<(), AppError> {
    window::manager::restore_from_snap(&app)
}

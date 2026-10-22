use tauri::{AppHandle, LogicalPosition, Manager};

use crate::error::AppError;

pub const LABEL_MAIN: &str = "main";
pub const LABEL_SETTINGS: &str = "settings";

pub fn hide_window(window: &tauri::Window) -> Result<(), AppError> {
    window.hide().map_err(AppError::from)
}

// TODO(C2): 托盘"显示/设置"菜单落地时补 `show_window`。

// ---- D4 边缘吸附（休眠态） ----

#[derive(serde::Serialize)]
pub struct MonitorView {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(serde::Serialize)]
pub struct OuterPositionView {
    pub x: i32,
    pub y: i32,
    pub monitor: MonitorView,
}

/// 窗口 A 外位置 + 当前显示器逻辑矩形（统一 logical 坐标，规避多屏 DPI 坑；D4.4 贴边判定数据源）。
pub fn outer_position_view(app: &AppHandle) -> Result<OuterPositionView, AppError> {
    let window = app
        .get_webview_window(LABEL_MAIN)
        .ok_or_else(|| AppError::Window(format!("窗口不存在: {LABEL_MAIN}")))?;
    let scale = window.scale_factor().map_err(AppError::from)?;
    let pos = window.outer_position().map_err(AppError::from)?.to_logical::<i32>(scale);
    let monitor = window
        .current_monitor()
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::Window("无法获取当前显示器".into()))?;
    let mp = monitor.position().to_logical::<i32>(scale);
    let ms = monitor.size().to_logical::<i32>(scale);
    Ok(OuterPositionView {
        x: pos.x,
        y: pos.y,
        monitor: MonitorView { x: mp.x, y: mp.y, width: ms.width, height: ms.height },
    })
}

/// 贴边目标位置**纯函数**（可测）：返回窗口左上角逻辑坐标。
/// edge ∈ top/left/right；显示器矩形 (mx,my,mw,mh)，窗口尺寸 (w,h)。
pub fn snap_target_position(
    edge: &str,
    m: (i32, i32, i32, i32),
    win: (i32, i32),
) -> Result<(i32, i32), AppError> {
    let (mx, my, mw, mh) = m;
    let (w, h) = win;
    let (x, y) = match edge {
        "top" => ((mx + mw / 2 - w / 2), my),
        "left" => (mx, (my + mh / 2 - h / 2)),
        "right" => (mx + mw - w, (my + mh / 2 - h / 2)),
        _ => return Err(AppError::Config(format!("未知 edge: {edge}"))),
    };
    Ok((x, y))
}

/// 贴边定位（Rust 经 current_monitor 计算边界 + set_position；窗口操作全 Rust，rule.md §5）。
///
/// 半透明（≈0.6）视觉降级由前端 PIXI 层 `setSleeping` 实现：
/// tauri 2.12.4 的 `WebviewWindow` 无 `set_opacity` API（子计划 D4 §五 风险#3 预留降级），
/// 直接调暗宠物角色 alpha 视觉等效且不引入 Win32 分层透明度兼容风险。
pub fn snap_to_edge(app: &AppHandle, edge: &str) -> Result<(), AppError> {
    let window = app
        .get_webview_window(LABEL_MAIN)
        .ok_or_else(|| AppError::Window(format!("窗口不存在: {LABEL_MAIN}")))?;
    let scale = window.scale_factor().map_err(AppError::from)?;
    let monitor = window
        .current_monitor()
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::Window("无法获取当前显示器".into()))?;
    let mp = monitor.position().to_logical::<i32>(scale);
    let ms = monitor.size().to_logical::<i32>(scale);
    let size = window.outer_size().map_err(AppError::from)?.to_logical::<i32>(scale);
    let (x, y) =
        snap_target_position(edge, (mp.x, mp.y, ms.width, ms.height), (size.width, size.height))?;
    window.set_position(LogicalPosition::new(x as f64, y as f64)).map_err(AppError::from)?;
    Ok(())
}

/// 解除吸附（位置由用户拖动决定，不自动回中）。
///
/// 半透明恢复由前端 `setSleeping(false)` 处理；本命令保留以对齐 D4 命令面（API 稳定），
/// 待 tauri 提供 `set_opacity` 后可在此补回窗口级透明度。
pub fn restore_from_snap(_app: &AppHandle) -> Result<(), AppError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_target_positions() {
        // 显示器 1920x1080 左上 (0,0)，窗口 400x400
        let win = (400, 400);
        let mon = (0, 0, 1920, 1080);
        assert_eq!(snap_target_position("top", mon, win).unwrap(), (760, 0)); // 居中贴顶
        assert_eq!(snap_target_position("left", mon, win).unwrap(), (0, 340)); // 左缘垂直居中
        assert_eq!(snap_target_position("right", mon, win).unwrap(), (1520, 340)); // 右缘垂直居中
        assert!(snap_target_position("bad", mon, win).is_err(), "非法 edge 报错");
    }
}

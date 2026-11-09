use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, Position};

use crate::error::AppError;

pub const LABEL_MAIN: &str = "main";
pub const LABEL_SETTINGS: &str = "settings";
/// 划词翻译独立浮窗（第三窗口；不参与点击穿透托管）。
pub const LABEL_TRANSLATOR: &str = "translator";

/// 窗口 A 常态尺寸（逻辑 px）：贴合形象——含尾巴摆动与伸懒腰形变余量，
/// 与 `tauri.conf.json` 的 main 窗口 width/height **必须一致**。
pub const PET_WINDOW_SIZE: (i32, i32) = (150, 150);
/// 窗口 A 展开态尺寸（逻辑 px）：气泡/输入框出现时临时放大，消失后收起。
pub const PET_WINDOW_EXPANDED: (i32, i32) = (420, 300);

pub fn hide_window(window: &tauri::Window) -> Result<(), AppError> {
    window.hide().map_err(AppError::from)
}

// ---- 划词翻译浮窗（第三窗口，全部由 Rust 定位/显隐） ----

fn translator_window(app: &AppHandle) -> Result<tauri::WebviewWindow, AppError> {
    app.get_webview_window(LABEL_TRANSLATOR)
        .ok_or_else(|| AppError::Window(format!("窗口不存在: {LABEL_TRANSLATOR}")))
}

/// 把浮窗左上角夹取到当前显示器内（**物理像素**，与钩子 `MSLLHOOKSTRUCT.pt` 同坐标系）。
///
/// 选区坐标来自全局鼠标钩子，是物理像素；若当逻辑像素传给 `set_position`，高 DPI 下
/// 窗口会被放大偏移、离选中词很远。`content_h` 是内容高度（逻辑像素，内部乘 scale）。
fn clamped_position(
    window: &tauri::WebviewWindow,
    x: i32,
    y: i32,
    content_h_logical: i32,
) -> (i32, i32) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let content_h = (content_h_logical as f64 * scale).round() as i32;
    let size = window.outer_size().map(|s| (s.width as i32, s.height as i32)).unwrap_or((380, 260));
    let Some(monitor) = window.current_monitor().ok().flatten() else {
        return (x, y);
    };
    let mp = monitor.position();
    let ms = monitor.size();
    let max_x = (mp.x + ms.width as i32 - size.0).max(mp.x);
    let max_y = (mp.y + ms.height as i32 - content_h.min(ms.height as i32)).max(mp.y);
    (x.clamp(mp.x, max_x), y.clamp(mp.y, max_y))
}

/// 展示内容高度（逻辑像素）：`compact=true` 为胶囊工具条，`false` 为译文卡片。
fn content_height(compact: bool) -> i32 {
    if compact { 60 } else { 240 }
}

/// 登记窗口物理矩形，供全局鼠标钩子判定「点击窗口之外」（与钩子 pt 同坐标系）。
fn register_translator_bounds(window: &tauri::WebviewWindow) {
    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
        crate::probe::selection::set_translator_bounds(
            pos.x,
            pos.y,
            size.width as i32,
            size.height as i32,
        );
    }
}

/// 在选中词附近显示浮窗（坐标由前端按选区包围盒算好；Rust 只做显示器夹取）。
pub fn show_translator(app: &AppHandle, x: i32, y: i32, compact: bool) -> Result<(), AppError> {
    let window = translator_window(app)?;
    let (x, y) = clamped_position(&window, x, y, content_height(compact));
    window.set_position(Position::Physical(PhysicalPosition::new(x, y))).map_err(AppError::from)?;
    window.show().map_err(AppError::from)?;
    register_translator_bounds(&window);
    Ok(())
}

/// 选区 / 鼠标移动时重定位浮窗（不改变可见性）。
pub fn move_translator(app: &AppHandle, x: i32, y: i32, compact: bool) -> Result<(), AppError> {
    let window = translator_window(app)?;
    let (x, y) = clamped_position(&window, x, y, content_height(compact));
    window.set_position(Position::Physical(PhysicalPosition::new(x, y))).map_err(AppError::from)?;
    register_translator_bounds(&window);
    Ok(())
}

/// 隐藏浮窗（关闭按钮 / Esc / 点击外部；窗口不销毁）。
pub fn hide_translator(app: &AppHandle) -> Result<(), AppError> {
    translator_window(app)?.hide().map_err(AppError::from)?;
    crate::probe::selection::clear_translator_bounds();
    Ok(())
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
    /// 窗口外尺寸（逻辑 px）：前端 right 贴边判定需右边缘 = x + width。
    pub width: i32,
    pub height: i32,
    pub monitor: MonitorView,
}

/// 窗口 A 外位置 + 外尺寸 + 当前显示器逻辑矩形（统一 logical 坐标，规避多屏 DPI 坑；D4.4 贴边判定数据源）。
pub fn outer_position_view(app: &AppHandle) -> Result<OuterPositionView, AppError> {
    let window = app
        .get_webview_window(LABEL_MAIN)
        .ok_or_else(|| AppError::Window(format!("窗口不存在: {LABEL_MAIN}")))?;
    let scale = window.scale_factor().map_err(AppError::from)?;
    let pos = window.outer_position().map_err(AppError::from)?.to_logical::<i32>(scale);
    let size = window.outer_size().map_err(AppError::from)?.to_logical::<i32>(scale);
    let monitor = window
        .current_monitor()
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::Window("无法获取当前显示器".into()))?;
    let mp = monitor.position().to_logical::<i32>(scale);
    let ms = monitor.size().to_logical::<i32>(scale);
    Ok(OuterPositionView {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
        monitor: MonitorView { x: mp.x, y: mp.y, width: ms.width, height: ms.height },
    })
}

/// 贴边目标位置**纯函数**（可测）：返回窗口左上角逻辑坐标。
///
/// **仅支持 left/right**（用户决策：上下不吸附）。
/// **纵向位置保留**用户拖到的位置（仅贴横向边缘），并 clamp 到显示器可见范围内
/// （拖拽可能把窗口带出屏幕上下缘）。
/// m = 显示器矩形 (mx,my,mw,mh)，win = 窗口尺寸 (w,h)，cur = 当前左上角 (x,y)。
pub fn snap_target_position(
    edge: &str,
    m: (i32, i32, i32, i32),
    win: (i32, i32),
    cur: (i32, i32),
) -> Result<(i32, i32), AppError> {
    let (mx, my, mw, mh) = m;
    let (w, h) = win;
    let (_cur_x, cur_y) = cur;
    let x = match edge {
        "left" => mx,
        "right" => mx + mw - w,
        _ => return Err(AppError::Config(format!("未知 edge（仅支持 left/right）: {edge}"))),
    };
    let y = clamp_to_monitor(cur_y, my, mh, h);
    Ok((x, y))
}

/// 把窗口纵向位置限制在显示器内（上缘 ≥ my，下缘 ≤ my+mh）。
fn clamp_to_monitor(y: i32, my: i32, mh: i32, h: i32) -> i32 {
    let max_y = (my + mh - h).max(my); // 显示器比窗口还矮时取 my
    y.clamp(my, max_y)
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
    let pos = window.outer_position().map_err(AppError::from)?.to_logical::<i32>(scale);
    let (x, y) = snap_target_position(
        edge,
        (mp.x, mp.y, ms.width, ms.height),
        (size.width, size.height),
        (pos.x, pos.y),
    )?;
    window.set_position(LogicalPosition::new(x as f64, y as f64)).map_err(AppError::from)?;
    Ok(())
}

/// 展开/收起的几何计算**纯函数**（可测）：返回 (目标尺寸, 左上角位移)。
///
/// 策略：**保持窗口中心不动** → 向四周等量扩展/收缩，窗口内形象（居中渲染）
/// 的屏幕位置保持不变，避免气泡出现时形象跳动。
pub fn bubble_resize_geometry(cur: (i32, i32), target: (i32, i32)) -> ((i32, i32), (i32, i32)) {
    let (cw, ch) = cur;
    let (tw, th) = target;
    ((tw, th), ((cw - tw) / 2, (ch - th) / 2))
}

/// 气泡/输入框展开态切换（Rust 实现窗口尺寸与位置，rule.md §5）。
///
/// 幂等：尺寸已为目标值则直接返回（气泡频繁切换时不产生抖动）。
pub fn set_bubble_mode(app: &AppHandle, expanded: bool) -> Result<(), AppError> {
    let window = app
        .get_webview_window(LABEL_MAIN)
        .ok_or_else(|| AppError::Window(format!("窗口不存在: {LABEL_MAIN}")))?;
    let scale = window.scale_factor().map_err(AppError::from)?;
    let cur = window.outer_size().map_err(AppError::from)?.to_logical::<i32>(scale);
    let target = if expanded { PET_WINDOW_EXPANDED } else { PET_WINDOW_SIZE };
    if (cur.width, cur.height) == target {
        return Ok(());
    }
    let ((w, h), (dx, dy)) = bubble_resize_geometry((cur.width, cur.height), target);
    let pos = window.outer_position().map_err(AppError::from)?.to_logical::<i32>(scale);
    window.set_size(LogicalSize::new(w as f64, h as f64)).map_err(AppError::from)?;
    window
        .set_position(LogicalPosition::new((pos.x + dx) as f64, (pos.y + dy) as f64))
        .map_err(AppError::from)?;
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
    fn bubble_resize_keeps_center() {
        // 常态 150x150 → 展开 420x300：中心不动 → 左上各移 -135 / -75
        assert_eq!(bubble_resize_geometry((150, 150), (420, 300)), ((420, 300), (-135, -75)));
        // 展开 → 收起：反向等量回移
        assert_eq!(bubble_resize_geometry((420, 300), (150, 150)), ((150, 150), (135, 75)));
    }

    #[test]
    fn bubble_resize_same_size_is_noop_offset() {
        // 已是目标尺寸 → 位移 0（幂等路径由 set_bubble_mode 提前返回）
        assert_eq!(bubble_resize_geometry((150, 150), (150, 150)), ((150, 150), (0, 0)));
    }

    #[test]
    fn snap_target_positions() {
        // 显示器 1920x1080 左上 (0,0)，窗口 150x150；仅支持 left/right（上下不吸附）
        let win = (150, 150);
        let mon = (0, 0, 1920, 1080);
        // 纵向位置**保留**用户拖到的位置（不再强制居中）
        assert_eq!(snap_target_position("left", mon, win, (900, 200)).unwrap(), (0, 200)); // 贴左缘，y 不变
        assert_eq!(
            snap_target_position("right", mon, win, (900, 700)).unwrap(),
            (1770, 700) // 右缘 = mx+mw-w = 1920-150；y 不变
        );
    }

    #[test]
    fn snap_target_clamps_vertical_into_monitor() {
        let win = (150, 150);
        let mon = (0, 0, 1920, 1080);
        // 拖出上缘 → clamp 到 my
        assert_eq!(snap_target_position("left", mon, win, (900, -80)).unwrap(), (0, 0));
        // 拖出下缘 → clamp 到 my+mh-h = 930
        assert_eq!(snap_target_position("right", mon, win, (900, 5000)).unwrap(), (1770, 930));
    }

    #[test]
    fn snap_target_rejects_vertical_edges() {
        let win = (150, 150);
        let mon = (0, 0, 1920, 1080);
        // 上下不吸附（用户决策）：top/bottom 一律报 Config 错误
        assert!(snap_target_position("top", mon, win, (900, 0)).is_err(), "top 已下线");
        assert!(snap_target_position("bottom", mon, win, (900, 0)).is_err(), "bottom 不支持");
        assert!(snap_target_position("bad", mon, win, (900, 0)).is_err(), "非法 edge 报错");
    }
}

//! 动态点击穿透（C2.4）：命中判定 + 状态翻转驱动。
//!
//! 性能红线：`set_ignore_cursor_events` **仅在状态翻转时调用**（每 100ms 轮询
//! 最多一次系统调用，且多数轮询零调用）；判定输入全部来自进程内缓存
//! （`AppState.hit_region` / `cursor`），不走事件。

use tauri::{AppHandle, Manager, WebviewWindow, Window};

use crate::error::AppError;
use crate::state::{AppState, HitRegion};

/// 命中判定（**纯函数**，单测覆盖）：cursor 是否落在窗口 A 的命中矩形内。
///
/// 坐标系：cursor 与 window_pos 均为**屏幕坐标**；hit_region 为窗口局部坐标。
/// 规则：
/// - 区域无效（未上报/尺寸为 0）→ 视为未命中（回退默认穿透，计划 §3.2 兜底）；
/// - 边界四边取闭区间（含边），与 `set_ignore_cursor_events` 的像素语义一致。
pub fn is_over_pet(cursor: (i32, i32), window_pos: (i32, i32), region: &HitRegion) -> bool {
    if !region.is_valid() {
        return false;
    }
    let (cx, cy) = cursor;
    let (wx, wy) = window_pos;
    let left = wx + region.x;
    let top = wy + region.y;
    cx >= left && cx <= left + region.width && cy >= top && cy <= top + region.height
}

/// 由缓存计算目标穿透状态（纯函数：true=穿透开）。
///
/// 总闸（`click_through_enabled=false`，D1 输入框/D4 休眠场景）→ 强制 false（整窗接收）。
pub fn target_ignore(over_pet: bool, click_through_enabled: bool) -> bool {
    click_through_enabled && !over_pet
}

/// 执行一次穿透同步：读缓存 → 判翻转 → 必要时调用系统 API。
///
/// 返回本次是否发生了系统调用（供测试/诊断）。
pub fn sync_click_through(app: &AppHandle, window: &WebviewWindow) -> Result<bool, AppError> {
    let Some(state) = app.try_state::<AppState>() else {
        return Ok(false);
    };
    let (over_pet, enabled) = {
        let region = *state.hit_region.lock().map_err(lock_err)?;
        let cursor = *state.cursor.lock().map_err(lock_err)?;
        let enabled = *state.click_through_enabled.lock().map_err(lock_err)?;
        let pos = window.outer_position().map_err(AppError::from)?;
        (is_over_pet(cursor, (pos.x, pos.y), &region), enabled)
    };
    let target = target_ignore(over_pet, enabled);

    let mut last = state.last_ignore.lock().map_err(lock_err)?;
    // 状态未变：零系统调用（性能红线）
    if *last != Some(target) {
        window.set_ignore_cursor_events(target)?;
        *last = Some(target);
        return Ok(true);
    }
    Ok(false)
}

/// 锁中毒映射（与 B5 命令层口径一致）。
fn lock_err<T>(e: T) -> AppError
where
    T: std::fmt::Display,
{
    AppError::Db(format!("获取状态锁失败: {e}"))
}

/// 窗口 A 的穿透状态复位（窗口隐藏/失焦时调用：强制回退穿透开）。
/// 由 lib.rs 的窗口事件接线（B1.2 关闭拦截与隐藏场景复用）；
/// 收 `&Window`：`on_window_event` 传入即该类型。
pub fn reset_to_passthrough(window: &Window) -> Result<(), AppError> {
    window.set_ignore_cursor_events(true).map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(x: i32, y: i32, w: i32, h: i32) -> HitRegion {
        HitRegion { x, y, width: w, height: h }
    }

    #[test]
    fn point_inside_region_is_over_pet() {
        let r = region(10, 20, 100, 80);
        // 窗口位于 (500,300) → 区域 = [510,610]x[320,400]
        assert!(is_over_pet((550, 350), (500, 300), &r));
    }

    #[test]
    fn point_outside_region_is_not_over_pet() {
        let r = region(10, 20, 100, 80);
        assert!(!is_over_pet((200, 350), (500, 300), &r));
        assert!(!is_over_pet((550, 100), (500, 300), &r));
    }

    #[test]
    fn boundary_is_inclusive() {
        let r = region(10, 20, 100, 80);
        // 左/上边界
        assert!(is_over_pet((510, 320), (500, 300), &r));
        // 右/下边界
        assert!(is_over_pet((610, 400), (500, 300), &r));
        // 超出右/下 1px
        assert!(!is_over_pet((611, 400), (500, 300), &r));
        assert!(!is_over_pet((610, 401), (500, 300), &r));
    }

    #[test]
    fn invalid_region_falls_back_to_passthrough() {
        // 未上报（全 0）→ 未命中 → 穿透开
        assert!(!is_over_pet((0, 0), (0, 0), &HitRegion::default()));
        assert!(!is_over_pet((100, 100), (0, 0), &region(0, 0, 0, 100)));
        assert!(!is_over_pet((100, 100), (0, 0), &region(0, 0, 100, 0)));
    }

    #[test]
    fn target_ignore_over_pet_means_clickable() {
        // 悬停猫身 → 不穿透（false）；透明区 → 穿透（true）
        assert!(!target_ignore(true, true));
        assert!(target_ignore(false, true));
    }

    #[test]
    fn master_switch_off_forces_clickable() {
        // 总闸关（输入框可见期间）→ 整窗接收，无论是否悬停猫身
        assert!(!target_ignore(false, false));
        assert!(!target_ignore(true, false));
    }

    #[test]
    fn negative_window_offset() {
        // 多屏/跨屏：窗口坐标为负（主屏左侧的副屏）换算正确
        let r = region(0, 0, 400, 400);
        assert!(is_over_pet((-200, -200), (-400, -400), &r));
        assert!(!is_over_pet((-401, -200), (-400, -400), &r));
    }
}

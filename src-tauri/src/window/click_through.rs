//! 动态点击穿透（C2.4）：命中判定 + 状态翻转驱动。
//!
//! 性能红线：`set_ignore_cursor_events` **仅在状态翻转时调用**（每 100ms 轮询
//! 最多一次系统调用，且多数轮询零调用）。判定输入：命中区来自进程内缓存
//! （`AppState.hit_region`），鼠标位置**每轮现取** `GetCursorPos`（见
//! `sync_click_through`——缓存由探针线程维护，快速移动后点击时最坏滞后两轮）。
//! 窗口移动静默窗内整轮跳过（`state::DRAG_QUIET_MS`），拖拽模态循环零系统调用。

use tauri::{AppHandle, Manager, Runtime, WebviewWindow, Window};

use crate::error::AppError;
use crate::state::{AppState, HitRegion};
use crate::window::manager::LABEL_MAIN;

/// 是否由穿透状态机托管（**纯函数**）：仅窗口 A（透明桌宠层）。
///
/// 窗口 B 是常规不透明窗口，`set_ignore_cursor_events` 对其等于"整窗点不动"；
/// 而 `start_click_through_sync` 只驱动窗口 A，B 一旦被置穿透便无任何复位路径，
/// 只能靠重启进程清除。故所有穿透写入都必须先过这道判定。
pub fn is_passthrough_managed(label: &str) -> bool {
    label == LABEL_MAIN
}

/// 命中判定（**纯函数**，单测覆盖）：cursor 是否落在窗口 A 的命中矩形内。
///
/// 坐标系：cursor、window_pos、region 三者均为**屏幕物理像素**（region 由
/// `window_set_hit_region` 命令在落缓存时单点换算，见该命令文档）。
/// 规则：
/// - 区域无效（首启未上报 / 吸附休眠裁切为 0 / 渲染器挂载失败）→ **视为命中，整窗可点击**。
///   绝不能回退成「整窗穿透」：`target_ignore` 会恒为 true，配合 `sync_click_through`
///   只在翻转时写系统 API 的短路，会形成"一旦打开就永不关闭"的死锁，用户再也点不到桌宠。
///   宁可短暂失去透明区穿透，也要保证形象始终可拖动 / 可点醒。
/// - 边界四边取闭区间（含边），与 `set_ignore_cursor_events` 的像素语义一致。
pub fn is_over_pet(cursor: (i32, i32), window_pos: (i32, i32), region: &HitRegion) -> bool {
    if !region.is_valid() {
        return true;
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

/// 记录一次窗口移动（`lib.rs` 的 `WindowEvent::Moved` 转发）。
///
/// 仅窗口 A 生效；系统拖拽逐帧触发，`AtomicU64::Relaxed` 即可——时间戳只用于
/// "静默窗内 / 外"判定，无跨变量顺序依赖。
pub fn note_window_moved<R: Runtime>(window: &Window<R>) {
    if !is_passthrough_managed(window.label()) {
        return;
    }
    if let Some(state) = window.try_state::<AppState>() {
        state.note_drag_moved();
    }
}

/// 执行一次穿透同步：读缓存 → 判翻转 → 必要时调用系统 API。
///
/// 仅窗口 A 受托管；传入其它窗口一律不写穿透（避免误伤设置窗口）。
/// 移动静默窗（系统拖拽 / `set_position` 后 300ms）内整轮跳过：模态拖拽循环里
/// cursor 缓存与命中区必然滞后，误判会把窗口置穿透、丢鼠标事件中断拖拽。
/// 返回本次是否发生了系统调用（供测试/诊断）。
pub fn sync_click_through<R: Runtime>(
    app: &AppHandle<R>,
    window: &WebviewWindow<R>,
) -> Result<bool, AppError> {
    if !is_passthrough_managed(window.label()) {
        return Ok(false);
    }
    let Some(state) = app.try_state::<AppState>() else {
        return Ok(false);
    };
    // 移动静默窗：跳过同步且**不动翻转缓存**——拖动结束后 last_ignore 仍为
    // start_drag 清下的 None，下一轮无条件重算自愈（不依赖前端回调）。
    if state.is_move_quiet() {
        return Ok(false);
    }
    let (over_pet, enabled) = {
        let region = *state.hit_region.lock().map_err(lock_err)?;
        let mut cursor = *state.cursor.lock().map_err(lock_err)?;
        // 现取鼠标位置：缓存来自探针线程（另一条 100ms 循环），快速移动到形象上
        // 点击时最坏滞后两轮（200ms）才解除穿透，点击落到下层窗口。GetCursorPos
        // 为轻量只读调用，本线程同样 100ms 一次，成本可忽略。
        if let Ok(live) = crate::probe::cursor::current_position() {
            cursor = live;
        }
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

/// 窗口隐藏时的穿透复位（由 `lib.rs` 的 `on_window_event` 调用）。
///
/// **仅窗口 A 生效**：此前对 A/B 一律 `set_ignore_cursor_events(true)`，
/// 导致设置窗口关闭后被永久穿透、重新 `show()` 也无法恢复（状态机只轮询 A），
/// 只能重启进程清除。常规窗口必须始终保持可点击。
/// 同时把 `last_ignore` 缓存对齐为 `Some(true)`，避免 A 重新显示时状态机
/// 因缓存与实际不一致而漏掉一次必要的翻转。
/// 收 `&Window`：`on_window_event` 传入即该类型。
pub fn reset_to_passthrough<R: Runtime>(window: &Window<R>) -> Result<(), AppError> {
    if !is_passthrough_managed(window.label()) {
        return Ok(());
    }
    window.set_ignore_cursor_events(true).map_err(AppError::from)?;
    if let Some(state) = window.try_state::<AppState>() {
        let mut last = state.last_ignore.lock().map_err(lock_err)?;
        *last = Some(true);
    }
    Ok(())
}

/// 显示常规窗口前确保其可点击（防御性自愈）。
///
/// 用于 `window_show` 命令与托盘 `reveal_window`：即使窗口因历史版本逻辑
/// 被置为穿透，也能在每次显示时复位，无需用户重启应用。
/// 窗口 A 交给状态机按命中区翻转，此处不干预。
pub fn ensure_interactive<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), AppError> {
    if is_passthrough_managed(window.label()) {
        return Ok(());
    }
    window.set_ignore_cursor_events(false).map_err(AppError::from)
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

    // 回归：首次启动拖动桌宠后整窗点击穿透、再也点不动（2026-11-11 手工反馈）。
    // 根因：首启命中区未上报 / 贴边休眠裁切成 0 → 区域无效 → 旧逻辑回退"整窗穿透"，
    // 而状态机只在翻转时写系统 API，target 恒 true 便永不关闭。
    #[test]
    fn invalid_region_falls_back_to_clickable() {
        // 未上报（全 0）/ 零宽 / 零高 → 一律视为命中 → 不穿透
        assert!(is_over_pet((0, 0), (0, 0), &HitRegion::default()));
        assert!(is_over_pet((100, 100), (0, 0), &region(0, 0, 0, 100)));
        assert!(is_over_pet((100, 100), (0, 0), &region(0, 0, 100, 0)));
        // 关键：即使上一轮已被置为穿透，无效区域也必须驱动状态机翻回可点击
        assert!(!target_ignore(is_over_pet((0, 0), (0, 0), &HitRegion::default()), true,));
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

    // 回归：关闭设置窗口再打开后整窗点击穿透（2026-11-07 手工回归发现）。
    // 根因是隐藏时对所有窗口一律置穿透，而状态机只轮询窗口 A，B 再无复位路径。
    #[test]
    fn only_main_window_is_passthrough_managed() {
        assert!(is_passthrough_managed(LABEL_MAIN));
        assert!(!is_passthrough_managed(crate::window::manager::LABEL_SETTINGS));
        assert!(!is_passthrough_managed("unknown"));
    }
}

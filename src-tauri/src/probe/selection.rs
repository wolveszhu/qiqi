//! 全局划词手势检测与「仅主动触发」取词。
//!
//! - 手势：`WH_MOUSE_LL` 回调只做 O(1) 阈值判定与置位，事件由 scheduler 的 100ms 轮询 emit；
//!   **自家进程窗口（桌宠 / 设置 / 翻译浮窗）上的按下不算划词**，否则拖动桌宠会被
//!   误判为拖选文本并弹出工具条；
//! - 取词（2026-11-16 零污染改造）：手势结束时只经 `uia.rs` 的 `TextPattern`
//!   **只读**判定有无选区，并把选区文本存进**内存槽**供用户点「翻译」时取用；
//!   探测与取词全程不读写剪贴板、不注入 `Ctrl+C`、不恢复前台窗口；
//! - 唯一允许写剪贴板的入口是用户主动点击「复制」/「复制译文」（`write_clipboard_text`）；
//! - 不把选中文本写入日志；控制台 / 终端窗口跳过（UIA 在该场景也多不可用）。
//!
//! 已知边界：终端、远程桌面、自绘 UI、部分 PDF 不支持 TextPattern，此时不弹工具条，
//! 返回可行动提示，用户可自行复制后使用；绝不降级到剪贴板注入。

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GA_ROOT, GetAncestor, GetClassNameW, GetForegroundWindow,
    GetMessageW, GetWindowThreadProcessId, HHOOK, MSG, MSLLHOOKSTRUCT, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_RBUTTONDOWN, WindowFromPoint,
};

use crate::config::{
    SELECTION_DEDUPE_MS, SELECTION_DOUBLE_CLICK_MS, SELECTION_DOUBLE_CLICK_RADIUS_PX,
    SELECTION_MAX_DRAG_MS, SELECTION_MIN_DRAG_MS, SELECTION_MIN_DRAG_PX,
};
use crate::db::models::{CapturedSelectionDto, SelectionRectDto, TranslateSelectionPayload};
use crate::error::AppError;
use crate::probe::uia;

// ---------- 钩子状态（回调只写原子量，不做 COM / HTTP / 日志 IO） ----------

static DOWN_X: AtomicI64 = AtomicI64::new(0);
static DOWN_Y: AtomicI64 = AtomicI64::new(0);
static DOWN_MS: AtomicI64 = AtomicI64::new(0);
static DRAGGING: AtomicBool = AtomicBool::new(false);
static PENDING: AtomicBool = AtomicBool::new(false);
static UP_X: AtomicI64 = AtomicI64::new(0);
static UP_Y: AtomicI64 = AtomicI64::new(0);
static LAST_EMIT_MS: AtomicI64 = AtomicI64::new(0);
static LAST_X: AtomicI64 = AtomicI64::new(0);
static LAST_Y: AtomicI64 = AtomicI64::new(0);
static HOOK_STARTED: AtomicBool = AtomicBool::new(false);
/// 内存选区槽的有效期：超时后点「翻译」给可行动错误，不降级到任何剪贴板取词。
const SELECTION_CACHE_TTL_MS: i64 = 10_000;

/// 纯函数：内存选区槽是否仍在有效期内（供单测，不依赖系统时间）。
fn selection_cache_valid(at: i64, now: i64) -> bool {
    now >= at && now - at <= SELECTION_CACHE_TTL_MS
}

// ---- translator 窗口物理矩形（供全局钩子判定「点击窗口之外」） ----

static WIN_X: AtomicI64 = AtomicI64::new(0);
static WIN_Y: AtomicI64 = AtomicI64::new(0);
static WIN_W: AtomicI64 = AtomicI64::new(0);
static WIN_H: AtomicI64 = AtomicI64::new(0);
static WIN_VISIBLE: AtomicBool = AtomicBool::new(false);
static DISMISS: AtomicBool = AtomicBool::new(false);

// ---- 双击选中词（两次左键按下，间隔短且位移小） ----

static LAST_CLICK_MS: AtomicI64 = AtomicI64::new(0);
static LAST_CLICK_X: AtomicI64 = AtomicI64::new(0);
static LAST_CLICK_Y: AtomicI64 = AtomicI64::new(0);
static DOUBLE_CLICK: AtomicBool = AtomicBool::new(false);
static DBL_X: AtomicI64 = AtomicI64::new(0);
static DBL_Y: AtomicI64 = AtomicI64::new(0);

/// 由 `window/manager` 在显示/移动浮窗时登记物理矩形（与钩子 `MSLLHOOKSTRUCT.pt` 同坐标系）。
pub fn set_translator_bounds(x: i32, y: i32, width: i32, height: i32) {
    WIN_X.store(x as i64, Ordering::Relaxed);
    WIN_Y.store(y as i64, Ordering::Relaxed);
    WIN_W.store(width as i64, Ordering::Relaxed);
    WIN_H.store(height as i64, Ordering::Relaxed);
    WIN_VISIBLE.store(true, Ordering::Relaxed);
}

/// 隐藏浮窗时清除矩形，避免窗口隐藏后误判点击。
pub fn clear_translator_bounds() {
    WIN_VISIBLE.store(false, Ordering::Relaxed);
    WIN_W.store(0, Ordering::Relaxed);
    WIN_H.store(0, Ordering::Relaxed);
}

/// 纯函数：物理像素点是否落在给定矩形内（便于单测）。
pub fn inside_rect(px: i32, py: i32, x: i32, y: i32, width: i32, height: i32) -> bool {
    px >= x && py >= y && px < x.saturating_add(width) && py < y.saturating_add(height)
}

/// 当前点击是否落在已显示的 translator 窗口内（钩子回调专用，只读原子量）。
fn is_inside_translator(px: i32, py: i32) -> bool {
    WIN_VISIBLE.load(Ordering::Relaxed)
        && inside_rect(
            px,
            py,
            WIN_X.load(Ordering::Relaxed) as i32,
            WIN_Y.load(Ordering::Relaxed) as i32,
            WIN_W.load(Ordering::Relaxed) as i32,
            WIN_H.load(Ordering::Relaxed) as i32,
        )
}

/// 纯函数：本次左键按下是否应**跳过**划词手势（便于单测）。
///
/// 自家窗口一律跳过：拖动桌宠的位移（≥8px、≥60ms）恰好满足 `is_drag`，
/// 若不排除，松手即被当成拖选文本弹出翻译工具条；点击设置窗口同理。
pub fn should_ignore_gesture(inside_translator: bool, own_window: bool) -> bool {
    inside_translator || own_window
}

/// 鼠标位置下的顶层窗口是否属于本进程（桌宠 / 设置 / 翻译浮窗均为同进程 WebView）。
///
/// 用 `WindowFromPoint` 而非 `GetForegroundWindow`：低级钩子回调发生在窗口激活之前，
/// 前台窗口可能仍是旧的；取鼠标命中窗口才准确。WebView2 子窗口经
/// `GetAncestor(GA_ROOT)` 归一到 Tauri 顶层窗口后再比对 PID。
unsafe fn is_own_window_at(px: i32, py: i32) -> bool {
    let hwnd = unsafe { WindowFromPoint(POINT { x: px, y: py }) };
    if hwnd.0.is_null() {
        return false;
    }
    let root = unsafe { GetAncestor(hwnd, GA_ROOT) };
    let target = if root.0.is_null() { hwnd } else { root };
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(target, Some(&mut pid));
    }
    pid == unsafe { GetCurrentProcessId() }
}

/// 纯函数：由按下/松开两点计算选区包围盒（供工具条贴合选中词定位）。
pub fn selection_rect(down_x: i32, down_y: i32, up_x: i32, up_y: i32) -> SelectionRectDto {
    SelectionRectDto {
        x: down_x.min(up_x),
        y: down_y.min(up_y),
        width: (down_x - up_x).abs(),
        height: (down_y - up_y).abs(),
    }
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

/// 纯函数：是否满足「拖选完成」的位移与时长阈值（便于单测，不依赖系统 API）。
pub fn is_drag(dx: i64, dy: i64, elapsed_ms: i64) -> bool {
    dx.abs().max(dy.abs()) >= SELECTION_MIN_DRAG_PX
        && (SELECTION_MIN_DRAG_MS..=SELECTION_MAX_DRAG_MS).contains(&elapsed_ms)
}

/// 纯函数：两次左键按下是否构成双击选词（间隔短、位移小；不依赖系统 API）。
pub fn is_double_click(elapsed_ms: i64, dx: i64, dy: i64) -> bool {
    (0..=SELECTION_DOUBLE_CLICK_MS).contains(&elapsed_ms)
        && dx.abs() <= SELECTION_DOUBLE_CLICK_RADIUS_PX
        && dy.abs() <= SELECTION_DOUBLE_CLICK_RADIUS_PX
}

/// 低级鼠标钩子回调：`HC_ACTION`（code=0）之外一律直接转发。
unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == 0 && lparam.0 != 0 {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let message = wparam.0 as u32;
        if message == WM_LBUTTONDOWN {
            let inside = is_inside_translator(info.pt.x, info.pt.y);
            let own = unsafe { is_own_window_at(info.pt.x, info.pt.y) };
            // 点击工具条 / 浮窗之外且工具条可见 → 收起（点在桌宠上同样算「窗口之外」）。
            if !inside && WIN_VISIBLE.load(Ordering::Relaxed) {
                DISMISS.store(true, Ordering::Relaxed);
            }
            if should_ignore_gesture(inside, own) {
                // 自家窗口：拖动桌宠、点击设置窗口都不算划词。同时清空双击计数，
                // 避免与之后第三方应用里的点击拼成误判双击、误注入 Ctrl+C。
                DRAGGING.store(false, Ordering::Relaxed);
                LAST_CLICK_MS.store(0, Ordering::Relaxed);
            } else {
                let now = now_ms();
                let last = LAST_CLICK_MS.load(Ordering::Relaxed);
                let dx = info.pt.x as i64 - LAST_CLICK_X.load(Ordering::Relaxed);
                let dy = info.pt.y as i64 - LAST_CLICK_Y.load(Ordering::Relaxed);
                if last > 0 && is_double_click(now - last, dx, dy) {
                    // 双击选中词：无需拖选位移，直接请求弹工具条；清零防止三击重复
                    DOUBLE_CLICK.store(true, Ordering::Relaxed);
                    DBL_X.store(info.pt.x as i64, Ordering::Relaxed);
                    DBL_Y.store(info.pt.y as i64, Ordering::Relaxed);
                    LAST_CLICK_MS.store(0, Ordering::Relaxed);
                } else {
                    LAST_CLICK_MS.store(now, Ordering::Relaxed);
                    LAST_CLICK_X.store(info.pt.x as i64, Ordering::Relaxed);
                    LAST_CLICK_Y.store(info.pt.y as i64, Ordering::Relaxed);
                }
                DOWN_X.store(info.pt.x as i64, Ordering::Relaxed);
                DOWN_Y.store(info.pt.y as i64, Ordering::Relaxed);
                DOWN_MS.store(now, Ordering::Relaxed);
                DRAGGING.store(true, Ordering::Relaxed);
            }
        } else if message == WM_RBUTTONDOWN && WIN_VISIBLE.load(Ordering::Relaxed) {
            if !is_inside_translator(info.pt.x, info.pt.y) {
                DISMISS.store(true, Ordering::Relaxed);
            }
        } else if message == WM_LBUTTONUP && DRAGGING.swap(false, Ordering::Relaxed) {
            let dx = info.pt.x as i64 - DOWN_X.load(Ordering::Relaxed);
            let dy = info.pt.y as i64 - DOWN_Y.load(Ordering::Relaxed);
            let elapsed = now_ms() - DOWN_MS.load(Ordering::Relaxed);
            if is_drag(dx, dy, elapsed) {
                UP_X.store(info.pt.x as i64, Ordering::Relaxed);
                UP_Y.store(info.pt.y as i64, Ordering::Relaxed);
                PENDING.store(true, Ordering::Relaxed);
            }
        }
    }
    unsafe { CallNextHookEx(HHOOK::default(), code, wparam, lparam) }
}

/// 安装全局低级鼠标钩子（进程生命周期内单例；钩子线程自带消息循环，不新增轮询线程）。
pub fn start_selection_hook() {
    if HOOK_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    // UIA STA 线程与鼠标钩子同生命周期；它在手势阶段提供只读选区判定。
    uia::start_uia_probe();
    std::thread::spawn(|| {
        let hook =
            unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE::default(), 0) };
        let Ok(hook) = hook else {
            eprintln!("[probe] 划词钩子安装失败，翻译工具条不可用");
            return;
        };
        let mut msg = MSG::default();
        loop {
            let ret = unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) };
            if ret.0 <= 0 {
                break;
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
    });
}

/// 由 `probe/scheduler` 的唯一 100ms 轮询调用：先收起、再处理双击 / 拖选并 emit。
pub fn poll_selection(app: &AppHandle) {
    // 顺序不能反：先关掉旧工具条，再显示本次手势产生的新工具条。
    if DISMISS.swap(false, Ordering::Relaxed) {
        clear_translator_bounds();
        let _ = crate::window::manager::hide_translator(app);
        let _ = app.emit("translate:selection:hide", ());
    }
    // 双击选中词：UIA 只读判定有选区才弹工具条（不碰剪贴板、不注入按键）。
    // 控制台窗口返回 false 直接跳过；rect 退化为点击点 0×0。
    if DOUBLE_CLICK.swap(false, Ordering::Relaxed) {
        PENDING.store(false, Ordering::Relaxed);
        let x = DBL_X.load(Ordering::Relaxed) as i32;
        let y = DBL_Y.load(Ordering::Relaxed) as i32;
        if probe_and_cache_selection() {
            emit_selection(app, x, y, SelectionRectDto { x, y, width: 0, height: 0 });
        }
        return;
    }
    if !PENDING.swap(false, Ordering::Relaxed) {
        return;
    }
    let x = UP_X.load(Ordering::Relaxed) as i32;
    let y = UP_Y.load(Ordering::Relaxed) as i32;
    // 用按下/松开两点的包围盒，工具条据此贴合选中词定位。
    let rect = selection_rect(
        DOWN_X.load(Ordering::Relaxed) as i32,
        DOWN_Y.load(Ordering::Relaxed) as i32,
        x,
        y,
    );
    // 与双击**同口径**：拖选也必须由 UIA 只读判定确有文本选区。
    // 拖动文件夹 / 窗口 / 图标 / 滚动条只产生位移、不产生文本选区 → 不弹工具条，
    // 且全程不读写剪贴板（零污染）。
    if probe_and_cache_selection() {
        emit_selection(app, x, y, rect);
    }
}

/// 统一去重并广播选区事件（拖选与双击共用；控制台窗口跳过）。
fn emit_selection(app: &AppHandle, x: i32, y: i32, rect: SelectionRectDto) {
    if is_console_window() {
        return;
    }
    let now = now_ms();
    let near_last = (x as i64 - LAST_X.load(Ordering::Relaxed)).abs() < 4
        && (y as i64 - LAST_Y.load(Ordering::Relaxed)).abs() < 4;
    if near_last && now - LAST_EMIT_MS.load(Ordering::Relaxed) < SELECTION_DEDUPE_MS {
        return;
    }
    LAST_EMIT_MS.store(now, Ordering::Relaxed);
    LAST_X.store(x as i64, Ordering::Relaxed);
    LAST_Y.store(y as i64, Ordering::Relaxed);
    let payload = TranslateSelectionPayload {
        selection_key: format!("{x}-{y}-{now}"),
        rect,
        cursor_x: x,
        cursor_y: y,
    };
    let _ = app.emit("translate:selection:show", payload);
}

/// 控制台 / 终端窗口跳过：`Ctrl+C` 在其中是中断信号，注入会打断用户进程。
fn is_console_window() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut buf = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut buf);
        if len <= 0 {
            return false;
        }
        let Some(name_slice) = buf.get(..len as usize) else {
            return false;
        };
        let name = String::from_utf16_lossy(name_slice);
        name.contains("Console")
            || name.contains("CASCADIA")
            || name.contains("Term")
            || name.contains("mintty")
    }
}

/// 内存选区槽：手势判定成功后保存 UIA 只读取得的文本，供用户点「翻译」时取用。
///
/// 2026-11-16 零污染改造：不再备份探测前剪贴板、不再把选中词写进剪贴板；
/// 唯一写剪贴板的入口是用户主动 `write_clipboard_text`。
struct GestureSelection {
    text: String,
    at: i64,
}

fn gesture_selection() -> &'static Mutex<Option<GestureSelection>> {
    static SLOT: OnceLock<Mutex<Option<GestureSelection>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// 结束本轮手势的捕获状态（只清内存，绝不读写剪贴板）。
///
/// 调用点：工具条隐藏、胶囊切译文卡片、新一轮手势开始前、用户主动复制译文 / 原文。
pub fn clear_gesture_selection() {
    if let Ok(mut slot) = gesture_selection().lock() {
        *slot = None;
    }
}

/// 用户点「翻译」后从内存选区槽取词：全程不读写剪贴板、不注入按键。
pub fn capture_selection(selection_key: &str) -> Result<CapturedSelectionDto, AppError> {
    if is_console_window() {
        return Err(AppError::Probe("当前窗口不支持取词".into()));
    }
    let slot = gesture_selection().lock().map_err(|_| AppError::Probe("取词状态异常".into()))?;
    let state = slot.as_ref().ok_or_else(uia::selection_unavailable_error)?;
    if !selection_cache_valid(state.at, now_ms()) {
        return Err(uia::selection_unavailable_error());
    }
    let text = state.text.trim().to_string();
    if text.is_empty() {
        return Err(uia::selection_unavailable_error());
    }
    Ok(build_captured(selection_key, text))
}

/// 组装取词结果（选区包围盒供工具条贴合定位）。
fn build_captured(selection_key: &str, text: String) -> CapturedSelectionDto {
    let rect = selection_rect(
        DOWN_X.load(Ordering::Relaxed) as i32,
        DOWN_Y.load(Ordering::Relaxed) as i32,
        UP_X.load(Ordering::Relaxed) as i32,
        UP_Y.load(Ordering::Relaxed) as i32,
    );
    CapturedSelectionDto {
        selection_key: selection_key.to_string(),
        text,
        rect,
        via: "uia".into(),
    }
}

/// 手势阶段 UIA 只读判定：有选区则把文本存进内存槽并返回 true。
///
/// 零污染契约（2026-11-16）：
/// - 不 `Clipboard::clear()`、不 `set_text()`、不 `simulate_ctrl_c()`；
/// - 不恢复前台窗口（UIA 直接读焦点元素，不需要把焦点切回来）；
/// - UIA 不可用 / 无选区 / 纯空白 → 返回 false，不弹工具条、不做任何降级；
/// - 同步跑在 scheduler 的 100ms 轮询里，单次硬超时见 `uia::probe_selection`。
pub fn probe_and_cache_selection() -> bool {
    clear_gesture_selection(); // 新一轮手势先丢旧槽；失败时也不留过期文本
    if is_console_window() {
        return false;
    }
    let Some(text) = uia::probe_selection() else {
        return false;
    };
    if text.trim().is_empty() {
        return false;
    }
    if let Ok(mut slot) = gesture_selection().lock() {
        *slot = Some(GestureSelection { text, at: now_ms() });
        true
    } else {
        false
    }
}

/// 复制译文 / 选中原文（用户主动点击；不经前端 clipboard API）。
///
/// 这是**唯一**允许写剪贴板的入口：用户明确要复制才写。主动复制后结束本轮内存槽，
/// 后续手势不应再复用它。本函数不做任何备份/还原。
pub fn write_clipboard_text(text: &str) -> Result<(), AppError> {
    clear_gesture_selection();
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| AppError::Probe(format!("剪贴板打开失败: {e}")))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| AppError::Probe(format!("写入剪贴板失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PROBE_INTERVAL_MS;

    #[test]
    fn drag_requires_enough_distance_and_time() {
        assert!(is_drag(20, 0, 100));
        assert!(is_drag(0, -20, 100));
        assert!(!is_drag(2, 0, 100), "位移不足");
        assert!(!is_drag(20, 0, 10), "时长不足");
        assert!(!is_drag(20, 0, SELECTION_MAX_DRAG_MS + 1), "超时长按不算划词");
    }

    #[test]
    fn selection_rect_covers_both_drag_directions() {
        let forward = selection_rect(100, 200, 160, 190);
        assert_eq!((forward.x, forward.y, forward.width, forward.height), (100, 190, 60, 10));
        let backward = selection_rect(160, 190, 100, 200);
        assert_eq!((backward.x, backward.y, backward.width, backward.height), (100, 190, 60, 10));
    }

    #[test]
    fn inside_rect_uses_exclusive_edges() {
        assert!(inside_rect(10, 10, 0, 0, 100, 50));
        assert!(!inside_rect(100, 10, 0, 0, 100, 50), "右边界排他");
        assert!(!inside_rect(10, 50, 0, 0, 100, 50), "下边界排他");
        assert!(!inside_rect(-1, 10, 0, 0, 100, 50));
    }

    #[test]
    fn selection_cache_valid_within_ttl_and_rejects_stale() {
        assert!(selection_cache_valid(1_000, 1_000));
        assert!(selection_cache_valid(1_000, 1_000 + SELECTION_CACHE_TTL_MS));
        assert!(!selection_cache_valid(1_000, 1_000 + SELECTION_CACHE_TTL_MS + 1));
        assert!(!selection_cache_valid(2_000, 1_000), "时钟回拨不算有效");
    }

    #[test]
    fn uia_probe_budget_keeps_cursor_polling_alive() {
        assert!(
            uia::UIA_PROBE_TIMEOUT_MS <= PROBE_INTERVAL_MS.saturating_mul(2),
            "UIA 探测同步跑在 scheduler 线程，不得超过两轮探针间隔"
        );
    }

    #[test]
    fn own_windows_never_start_a_selection_gesture() {
        assert!(should_ignore_gesture(false, true), "拖动桌宠 / 点击设置窗口不算划词");
        assert!(should_ignore_gesture(true, false), "点击翻译工具条不算划词");
        assert!(should_ignore_gesture(true, true));
        assert!(!should_ignore_gesture(false, false), "第三方应用划词照常触发");
    }

    #[test]
    fn double_click_requires_short_interval_and_small_distance() {
        assert!(is_double_click(120, 1, -2));
        assert!(is_double_click(SELECTION_DOUBLE_CLICK_MS, 0, 0), "含上界");
        assert!(!is_double_click(SELECTION_DOUBLE_CLICK_MS + 1, 0, 0), "间隔过长");
        assert!(!is_double_click(120, 20, 0), "位移过大");
        assert!(!is_double_click(-1, 0, 0), "时钟回拨不算双击");
    }
}

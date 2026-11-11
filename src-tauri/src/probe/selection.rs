//! 全局划词手势检测与「仅主动触发」取词。
//!
//! - 手势：`WH_MOUSE_LL` 回调只做 O(1) 阈值判定与置位，事件由 scheduler 的 100ms 轮询 emit；
//!   **自家进程窗口（桌宠 / 设置 / 翻译浮窗）上的按下不算划词**，否则拖动桌宠会被
//!   误判为拖选文本并弹出工具条；
//! - 取词：翻译**只在用户点击「翻译」后**发生；但双击 / 拖选手势阶段会各自注入一次
//!   `Ctrl+C` 探测，仅用于判定「是否确有选中文本」（短等待、不抢焦点）。
//!   非文本手势立即还原剪贴板（不占用粘贴板）；文本手势的选中词**保留在剪贴板**，
//!   工具条消失也不清除——用户可能只是想复制这个词而不是翻译它；
//! - 红线：探测之外不在手势阶段读剪贴板 / 注入按键（探测只判定是否弹工具条，不翻译）；
//!   不把选中文本写入日志；控制台窗口跳过。
//!
//! 已知边界：本版以剪贴板法为唯一取词路径（UIA `TextPattern` 主路径待接入），
//! 目标窗口以管理员权限运行时注入按键受 UIPI 拦截，返回可行动的错误提示。

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_C, VK_CONTROL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GA_ROOT, GetAncestor, GetClassNameW, GetForegroundWindow,
    GetMessageW, GetWindowThreadProcessId, HHOOK, MSG, MSLLHOOKSTRUCT, SetForegroundWindow,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_RBUTTONDOWN, WindowFromPoint,
};

use crate::config::{
    SELECTION_CLIPBOARD_TRIES, SELECTION_CLIPBOARD_WAIT_MS, SELECTION_DEDUPE_MS,
    SELECTION_DOUBLE_CLICK_MS, SELECTION_DOUBLE_CLICK_RADIUS_PX, SELECTION_MAX_DRAG_MS,
    SELECTION_MIN_DRAG_MS, SELECTION_MIN_DRAG_PX, SELECTION_PROBE_TRIES, SELECTION_PROBE_WAIT_MS,
};
use crate::db::models::{CapturedSelectionDto, SelectionRectDto, TranslateSelectionPayload};
use crate::error::AppError;

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
/// 拖选开始时记录的前台窗口：点击工具条后需恢复它，否则 `Ctrl+C` 会发给自家浮窗。
static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);

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
                TARGET_HWND.store(unsafe { GetForegroundWindow() }.0 as isize, Ordering::Relaxed);
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
    // 双击选中词：仅当确有选中文本才弹工具条（取词探测 Ctrl+C 读剪贴板，空则 false）。
    // 控制台窗口返回 false 直接跳过；rect 退化为点击点 0×0。
    if DOUBLE_CLICK.swap(false, Ordering::Relaxed) {
        PENDING.store(false, Ordering::Relaxed);
        let x = DBL_X.load(Ordering::Relaxed) as i32;
        let y = DBL_Y.load(Ordering::Relaxed) as i32;
        if has_selected_text() {
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
    // 与双击**同口径**：拖选也必须验证确有选中文本。
    // 拖动文件夹 / 窗口 / 图标 / 滚动条只产生位移，不产生文本选中，
    // 若只凭 is_drag 的位移与时长阈值就 emit，会在这些场景误弹翻译工具条。
    if has_selected_text() {
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

/// 取词互斥锁：避免并发取词互相污染剪贴板。
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// 手势探测捕获的选中文本（工具条显示期间有效）。
///
/// **只记录文本供「点击翻译 / 复制」时复用**，不记录探测前的剪贴板内容：
/// 用户可能只想复制选中词而不是翻译它，工具条消失后剪贴板必须原样保留。
struct GestureClipboard {
    /// 本次捕获的原文（未 trim），用于判断剪贴板是否仍是我们放进去的内容。
    captured: String,
}

fn gesture_clipboard() -> &'static Mutex<Option<GestureClipboard>> {
    static SLOT: OnceLock<Mutex<Option<GestureClipboard>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// 纯函数：剪贴板当前内容是否仍是手势探测放进去的文本（只有是，才允许复用取词）。
fn still_owns_clipboard(current: Option<&str>, captured: &str) -> bool {
    current == Some(captured)
}

/// 把剪贴板恢复成探测前的文本；`None` 表示原本没有文本，清空剪贴板。
///
/// **仅用于「非文本手势」**：拖文件 / 文件夹时 `Ctrl+C` 可能把文件列表写进剪贴板，
/// 必须还原；文本手势绝不调用（见 [`GestureClipboard`] 文档）。
fn restore_text(clipboard: &mut arboard::Clipboard, previous: Option<&str>) {
    match previous {
        Some(prev) => {
            let _ = clipboard.set_text(prev.to_string());
        },
        None => {
            let _ = clipboard.clear();
        },
    }
}

/// 结束本轮手势的捕获状态（**只丢状态，绝不改动剪贴板**）。
///
/// 调用点：工具条隐藏、胶囊切译文卡片、新一轮手势开始前、用户主动复制译文 / 原文。
/// 用户决策（2026-11-11）：选中词留在剪贴板供直接 `Ctrl+V`，工具条消失也不清除——
/// 划词只是「可能要翻译」，用户完全可能只是想复制这个词。
pub fn clear_gesture_clipboard() {
    if let Ok(mut slot) = gesture_clipboard().lock() {
        *slot = None;
    }
}

/// 若手势探测的文本仍在剪贴板里，返回它（用户点「翻译」/「复制」时直接复用，
/// 不再注入 `Ctrl+C`：否则内容不变会被 `is_new_selection_text` 判为"无新文本"而取词失败）。
fn gesture_captured_text() -> Option<String> {
    let slot = gesture_clipboard().lock().ok()?;
    let state = slot.as_ref()?;
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let current = clipboard.get_text().ok()?;
    still_owns_clipboard(Some(current.as_str()), &state.captured).then(|| state.captured.clone())
}

/// 点击工具条后恢复原前台窗口，确保 `Ctrl+C` 发给第三方应用而非自家浮窗。
///
/// `SetForegroundWindow` 受前台锁定 / UIPI 限制，失败时不阻断：
/// 后续剪贴板读取若未拿到新内容会给用户可行动的提示。
fn restore_foreground() {
    let raw = TARGET_HWND.load(Ordering::Relaxed);
    if raw == 0 {
        return;
    }
    let target = HWND(raw as *mut std::ffi::c_void);
    unsafe {
        if GetForegroundWindow().0 != target.0 {
            let _ = SetForegroundWindow(target);
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}

/// 模拟一次 `Ctrl+C`（仅在用户点击小图标后调用）。
fn simulate_ctrl_c() {
    fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
    let inputs = [key(VK_CONTROL, false), key(VK_C, false), key(VK_C, true), key(VK_CONTROL, true)];
    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// 点击工具条后取词：优先复用手势探测已放进剪贴板的选中文本；否则走完整预算重新取词。
pub fn capture_selection(selection_key: &str) -> Result<CapturedSelectionDto, AppError> {
    let _guard = capture_lock().lock().map_err(|_| AppError::Probe("取词互斥锁异常".into()))?;
    if is_console_window() {
        return Err(AppError::Probe("当前窗口不支持取词".into()));
    }
    // 工具条显示期间剪贴板里就是探测捕获的原文，直接复用。若再注入 Ctrl+C，
    // 剪贴板内容不变会被 `is_new_selection_text` 判为"无新文本"而取词失败。
    let text = match gesture_captured_text() {
        Some(text) => text,
        None => capture_with_wait(SELECTION_CLIPBOARD_TRIES, SELECTION_CLIPBOARD_WAIT_MS)?,
    };
    // 复用路径拿到的是剪贴板原文，与重新取词路径保持同样的 trim 口径。
    Ok(build_captured(selection_key, text.trim().to_string()))
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
        via: "clipboard".into(),
    }
}

/// 手势阶段判定「是否确有选中文本」（双击 / 拖选共用，只返回 bool，不暴露文本）。
///
/// 剪贴板生命周期刻意分两种：
/// - **命中文本** → 选中词**留在剪贴板**：用户可能只是想复制而不是翻译，工具条显示期间
///   和消失后都可直接 `Ctrl+V`。工具条消失只丢弃捕获状态，绝不清剪贴板；
/// - **未命中**（拖文件夹 / 窗口 / 图标等）→ **立即还原剪贴板**。这些操作压根不该"复制"，
///   绝不能把 Explorer 因 `Ctrl+C` 写入的文件列表留在剪贴板里占用粘贴板。
///
/// 探测前先把剪贴板清空作哨兵：上一轮手势留下的选中词会成为本轮的比对基准，
/// 若用户再次选中同一段文本，`Ctrl+C` 后内容不变会被误判为"没有选中文本"而不弹工具条。
/// 未命中时按探测前原文还原，因此哨兵不会造成净改动。
///
/// 另外保持轻量：短预算（6×30ms）且不 `SetForegroundWindow`（手势刚结束时前台本就是
/// 目标窗口）。本函数同步跑在 scheduler 唯一的 100ms 轮询线程里，长时间阻塞会冻结
/// `state.cursor` 缓存，连带让穿透状态机用过期坐标误判。
pub fn has_selected_text() -> bool {
    let Ok(_guard) = capture_lock().lock() else {
        return false;
    };
    if is_console_window() {
        return false;
    }
    // 新一轮手势先丢弃上一轮的捕获状态（**不动剪贴板**，选中词要留给用户）。
    clear_gesture_clipboard();
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return false;
    };
    // 备份探测前内容，仅用于「未命中」时还原；文本手势不还原。
    let previous = clipboard.get_text().ok();
    // 清空作哨兵，使本次 Ctrl+C 产生的任何文本都能被判为"新文本"（含与上次相同的词）。
    let _ = clipboard.clear();
    // clear 可能失败（剪贴板被其它进程占用），此时以实际读回值作基准，
    // 避免把旧内容误判成本次选中词。
    let baseline = clipboard.get_text().ok().filter(|s| !s.trim().is_empty());

    simulate_ctrl_c();
    let mut captured: Option<String> = None;
    for _ in 0..SELECTION_PROBE_TRIES {
        std::thread::sleep(Duration::from_millis(SELECTION_PROBE_WAIT_MS));
        let current = clipboard.get_text().ok();
        if is_new_selection_text(baseline.as_deref(), current.as_deref()) {
            captured = current;
            break;
        }
    }
    match captured {
        Some(text) => {
            if let Ok(mut slot) = gesture_clipboard().lock() {
                *slot = Some(GestureClipboard { captured: text });
            }
            true
        },
        None => {
            restore_text(&mut clipboard, previous.as_deref());
            false
        },
    }
}

/// 纯函数：剪贴板文本是否构成一次有效的「复制到新文本」（供单测）。
///
/// 排除三种情况：`get_text()` 失败（拖动文件夹时剪贴板是文件列表等非文本格式）、
/// 与原内容相同（Ctrl+C 未生效）、纯空白（无实际选中内容）。
fn is_new_selection_text(previous: Option<&str>, current: Option<&str>) -> bool {
    match current {
        Some(text) => !text.trim().is_empty() && Some(text) != previous,
        None => false,
    }
}

/// 真正取词（仅当剪贴板里没有可复用的手势捕获文本时走）：
/// 备份 → 恢复原前台窗口 → 注入复制 → 等待更新 → **立即恢复**剪贴板。
fn capture_with_wait(tries: u32, wait_ms: u64) -> Result<String, AppError> {
    restore_foreground();
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| AppError::Probe(format!("剪贴板打开失败: {e}")))?;
    let previous = clipboard.get_text().ok();

    simulate_ctrl_c();
    let mut captured: Option<String> = None;
    for _ in 0..tries {
        std::thread::sleep(Duration::from_millis(wait_ms));
        let current = clipboard.get_text().ok();
        if is_new_selection_text(previous.as_deref(), current.as_deref()) {
            captured = current;
            break;
        }
    }
    // 取词完成即恢复，不在剪贴板留痕；捕获不到文本时保持原样，
    // 避免把用户已复制的非文本内容冲成空文本。
    if captured.is_some() {
        restore_text(&mut clipboard, previous.as_deref());
    }

    captured
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AppError::Probe("未能读取选中文本".into()))
}

/// 复制译文 / 选中原文（用户主动点击；不经前端 clipboard API）。
///
/// 主动复制后结束本轮捕获状态：剪贴板里现在是用户明确要的内容（译文或选中原文），
/// 后续手势不应再复用它。本函数只丢状态，不做任何还原。
pub fn write_clipboard_text(text: &str) -> Result<(), AppError> {
    clear_gesture_clipboard();
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
    fn clipboard_change_counts_only_as_new_non_blank_text() {
        assert!(is_new_selection_text(Some("old"), Some("selected")));
        // 探测前清空哨兵的基准：上一轮留下的选中词与本轮相同，也仍应判为"有新文本"，
        // 否则用户再次选中同一个词时工具条不会弹出。
        assert!(is_new_selection_text(None, Some("selected")));
        assert!(!is_new_selection_text(Some("old"), Some("old")), "Ctrl+C 未生效");
        assert!(!is_new_selection_text(Some("old"), None), "非文本格式（拖文件夹）");
        assert!(!is_new_selection_text(Some("old"), Some("  \n ")), "纯空白");
    }

    #[test]
    fn clipboard_reuse_only_when_we_still_own_it() {
        assert!(still_owns_clipboard(Some("selected"), "selected"));
        assert!(
            !still_owns_clipboard(Some("user copied something"), "selected"),
            "用户自己复制了别的内容时，不能再复用上次捕获的文本"
        );
        assert!(
            !still_owns_clipboard(None, "selected"),
            "剪贴板变成非文本格式（文件列表 / 图片）时也不能复用"
        );
    }

    #[test]
    fn gesture_probe_budget_keeps_cursor_polling_alive() {
        let gesture_ms = SELECTION_PROBE_TRIES as u64 * SELECTION_PROBE_WAIT_MS;
        let capture_ms = SELECTION_CLIPBOARD_TRIES as u64 * SELECTION_CLIPBOARD_WAIT_MS;
        assert!(
            gesture_ms <= PROBE_INTERVAL_MS.saturating_mul(2),
            "手势探测同步跑在 scheduler 线程，不得超过两轮探针间隔: {gesture_ms}ms"
        );
        assert!(gesture_ms < capture_ms, "手势判定应比真正取词更短");
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

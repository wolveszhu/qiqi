//! 全局划词手势检测与「仅主动触发」取词。
//!
//! - 手势：`WH_MOUSE_LL` 回调只做 O(1) 阈值判定与置位，事件由 scheduler 的 100ms 轮询 emit；
//! - 取词：**只在用户点击小图标后**执行；注入 `Ctrl+C` 前备份剪贴板、读完恢复、全程互斥；
//! - 红线：手势阶段不读剪贴板、不注入按键；不把选中文本写入日志；控制台窗口跳过。
//!
//! 已知边界：本版以剪贴板法为唯一取词路径（UIA `TextPattern` 主路径待接入），
//! 目标窗口以管理员权限运行时注入按键受 UIPI 拦截，返回可行动的错误提示。

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_C, VK_CONTROL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetClassNameW, GetForegroundWindow, GetMessageW, HHOOK, MSG,
    MSLLHOOKSTRUCT, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP,
};

use crate::config::{
    SELECTION_CLIPBOARD_TRIES, SELECTION_CLIPBOARD_WAIT_MS, SELECTION_DEDUPE_MS,
    SELECTION_MAX_DRAG_MS, SELECTION_MIN_DRAG_MS, SELECTION_MIN_DRAG_PX,
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

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

/// 纯函数：是否满足「拖选完成」的位移与时长阈值（便于单测，不依赖系统 API）。
pub fn is_drag(dx: i64, dy: i64, elapsed_ms: i64) -> bool {
    dx.abs().max(dy.abs()) >= SELECTION_MIN_DRAG_PX
        && (SELECTION_MIN_DRAG_MS..=SELECTION_MAX_DRAG_MS).contains(&elapsed_ms)
}

/// 低级鼠标钩子回调：`HC_ACTION`（code=0）之外一律直接转发。
unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == 0 && lparam.0 != 0 {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let message = wparam.0 as u32;
        if message == WM_LBUTTONDOWN {
            DOWN_X.store(info.pt.x as i64, Ordering::Relaxed);
            DOWN_Y.store(info.pt.y as i64, Ordering::Relaxed);
            DOWN_MS.store(now_ms(), Ordering::Relaxed);
            DRAGGING.store(true, Ordering::Relaxed);
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

/// 由 `probe/scheduler` 的唯一 100ms 轮询调用：有完成的选区才 emit，避免事件洪泛。
pub fn poll_selection(app: &AppHandle) {
    if !PENDING.swap(false, Ordering::Relaxed) {
        return;
    }
    if is_console_window() {
        return;
    }
    let x = UP_X.load(Ordering::Relaxed) as i32;
    let y = UP_Y.load(Ordering::Relaxed) as i32;
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
        rect: SelectionRectDto { x, y, width: 0, height: 0 },
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

/// 点击小图标后取词：备份 → 注入复制 → 等待剪贴板更新 → **无条件恢复**原内容。
pub fn capture_selection(selection_key: &str) -> Result<CapturedSelectionDto, AppError> {
    let _guard = capture_lock().lock().map_err(|_| AppError::Probe("取词互斥锁异常".into()))?;
    if is_console_window() {
        return Err(AppError::Config("当前窗口不支持取词".into()));
    }
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| AppError::Probe(format!("剪贴板打开失败: {e}")))?;
    let previous = clipboard.get_text().ok();

    simulate_ctrl_c();
    let mut captured: Option<String> = None;
    for _ in 0..SELECTION_CLIPBOARD_TRIES {
        std::thread::sleep(Duration::from_millis(SELECTION_CLIPBOARD_WAIT_MS));
        let current = clipboard.get_text().ok();
        if current.is_some() && current != previous {
            captured = current;
            break;
        }
    }
    // 无论成功与否都恢复用户剪贴板；原本没有文本时置空，避免残留本次复制内容。
    let restored = previous.unwrap_or_default();
    let _ = clipboard.set_text(restored);

    let text = captured
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AppError::Config("未能读取选中文本".into()))?;
    let (x, y) = (UP_X.load(Ordering::Relaxed) as i32, UP_Y.load(Ordering::Relaxed) as i32);
    Ok(CapturedSelectionDto {
        selection_key: selection_key.to_string(),
        text,
        rect: SelectionRectDto { x, y, width: 0, height: 0 },
        via: "clipboard".into(),
    })
}

/// 复制译文（用户主动点击；不经前端 clipboard API）。
pub fn write_clipboard_text(text: &str) -> Result<(), AppError> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| AppError::Probe(format!("剪贴板打开失败: {e}")))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| AppError::Probe(format!("写入剪贴板失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_requires_enough_distance_and_time() {
        assert!(is_drag(20, 0, 100));
        assert!(is_drag(0, -20, 100));
        assert!(!is_drag(2, 0, 100), "位移不足");
        assert!(!is_drag(20, 0, 10), "时长不足");
        assert!(!is_drag(20, 0, SELECTION_MAX_DRAG_MS + 1), "超时长按不算划词");
    }
}

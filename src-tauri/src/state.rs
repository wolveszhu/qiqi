//! 全局状态容器：命令层经 `tauri::State` 访问（规范 §4）。
//!
//! - `db`：SQLCipher 连接（B4 注入，setup 内 `app.manage`）；
//! - `hit_region`/`cursor`/`last_ignore`：C2 穿透状态机的进程内缓存（**不走事件**，零 IPC 洪泛）；
//! - `click_through_enabled`：穿透总闸（D1 输入框可见 / D4 边缘休眠时关闭）；
//! - `sleep_mode`：边缘吸附休眠态（D4.4，**运行时内存态、不落库**；日记等后台任务据此静默）；
//! - `drag_last_moved_ms`：窗口移动静默窗（C2.4，见 [`DRAG_QUIET_MS`]——拖拽模态循环里
//!   穿透误判会把窗口置穿透、丢鼠标事件中断拖拽，静默窗内穿透同步整轮跳过）。

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

/// 窗口移动静默窗（毫秒）：移动（系统拖拽 / `set_position`）后此时间内穿透状态机跳过同步。
///
/// 系统拖拽是模态循环，cursor 缓存（探针线程 100ms）与命中区（前端 500ms 上报）必然滞后，
/// 期间误判"未命中"会把窗口置穿透——鼠标消息穿透窗口导致拖拽中断、形象掉落；
/// 且 `window_start_drag` 已清翻转缓存，静默窗内每轮都会写系统 API（违反"仅翻转时调用"红线）。
/// `set_position`（吸附贴边 / 气泡展开）同样触发 `WindowEvent::Moved`，共享此缓冲。
pub const DRAG_QUIET_MS: u64 = 300;

/// 当前 UNIX 毫秒时间戳（静默窗判定用；时钟早于 epoch 时按 0 处理，配合 [`is_move_recent`] 的 0 短路）。
fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// 纯函数：`last_moved_ms` 之后 `DRAG_QUIET_MS` 内视为"窗口正在移动"（0 = 从未移动，恒 false）。
///
/// 时钟回拨（now < last）按**已过期**处理：穿透同步是自愈机制，宁可多跑也不停摆。
pub fn is_move_recent(last_moved_ms: u64, now_ms: u64) -> bool {
    last_moved_ms != 0 && now_ms >= last_moved_ms && now_ms - last_moved_ms < DRAG_QUIET_MS
}

/// 命中区域（**窗口局部坐标**；前端按 `getHitRegion()` 上报，变化时 invoke）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HitRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl HitRegion {
    /// 区域是否有效（未上报 / 尺寸为 0 视为无效 → 回退**整窗可点击**，绝不回退穿透）。
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// 应用级状态：经 `app.manage(AppState)` 注册，命令层经 `State<AppState>` 访问。
pub struct AppState {
    /// SQLCipher 连接（B4 注入；C6 穿透状态机的 cursor/hit_region 见下）。
    pub db: Mutex<Connection>,
    /// 命中区域缓存（C2.2：前端低频上报写入）。
    pub hit_region: Mutex<HitRegion>,
    /// 鼠标屏幕坐标缓存（C2.3：探针单点轮询写入）。
    pub cursor: Mutex<(i32, i32)>,
    /// 上一次应用到窗口的穿透状态（C2.4：仅翻转时调用系统 API）。
    pub last_ignore: Mutex<Option<bool>>,
    /// 穿透总闸：true=启用自动穿透（默认）；false=整窗接收点击（D1/D4 场景）。
    pub click_through_enabled: Mutex<bool>,
    /// 空闲状态机当前值（C6：true=idle；scheduler 翻转时 emit，省事件洪泛）。
    pub idle_active: Mutex<bool>,
    /// 前台进程名缓存（C6：变化时才 emit `probe:foreground:changed`）。
    pub last_foreground: Mutex<Option<String>>,
    /// 上一轮对话响应耗时（毫秒，E3.5：超时则下一轮注入慢回复歉意基调；内存态，重启清零）。
    pub last_response_ms: Mutex<Option<u64>>,
    /// 边缘吸附休眠态（D4.4：true=休眠中；日记生成等后台任务门控；内存态，重启清零）。
    pub sleep_mode: Mutex<bool>,
    /// 当前语言（启动读取 app_settings；`set_locale` 命令同步；托盘菜单刷新文案用）。
    pub locale: Mutex<String>,
    /// 最近一次窗口移动时刻（UNIX 毫秒；0=从未移动；原子量——`WindowEvent::Moved`
    /// 逐帧高频写入，见 [`DRAG_QUIET_MS`] 的静默窗纪律）。
    pub drag_last_moved_ms: AtomicU64,
}

impl AppState {
    /// 构造初始状态（db 由调用方注入；locale 一并缓存，托盘菜单刷新用）。
    pub fn new(db: Mutex<Connection>, locale: String) -> Self {
        Self {
            db,
            hit_region: Mutex::new(HitRegion::default()),
            cursor: Mutex::new((0, 0)),
            last_ignore: Mutex::new(None),
            click_through_enabled: Mutex::new(true),
            idle_active: Mutex::new(false),
            last_foreground: Mutex::new(None),
            last_response_ms: Mutex::new(None),
            sleep_mode: Mutex::new(false),
            locale: Mutex::new(locale),
            drag_last_moved_ms: AtomicU64::new(0),
        }
    }

    /// 是否处于边缘吸附休眠态（锁 poisoning 时按未休眠处理——宁可多跑一次后台任务）。
    pub fn is_sleep_mode(&self) -> bool {
        self.sleep_mode.lock().map(|g| *g).unwrap_or(false)
    }

    /// 记录一次窗口移动（`WindowEvent::Moved` 与 `window_start_drag` 调用）。
    pub fn note_drag_moved(&self) {
        self.drag_last_moved_ms.store(now_unix_ms(), Ordering::Relaxed);
    }

    /// 是否处于移动静默窗内（穿透同步据此整轮跳过，锁自由读）。
    pub fn is_move_quiet(&self) -> bool {
        is_move_recent(self.drag_last_moved_ms.load(Ordering::Relaxed), now_unix_ms())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::new(
            Mutex::new(rusqlite::Connection::open_in_memory().expect("内存库")),
            "zh-CN".to_string(),
        )
    }

    #[test]
    fn sleep_mode_defaults_false_and_toggles() {
        let s = state();
        assert!(!s.is_sleep_mode(), "默认未休眠（日记等后台任务正常运行）");
        *s.sleep_mode.lock().expect("锁") = true;
        assert!(s.is_sleep_mode(), "置位后应报告休眠（日记门控据此跳过）");
        *s.sleep_mode.lock().expect("锁") = false;
        assert!(!s.is_sleep_mode(), "复位后恢复");
    }

    // 回归：拖动桌宠期间穿透状态机仍按滞后缓存翻转，拖拽模态循环里被置穿透会
    // 丢鼠标事件导致拖拽中断（2026-11-11 手工反馈）。静默窗必须只在窗口移动后
    // DRAG_QUIET_MS 内生效，过期即恢复同步。
    #[test]
    fn move_quiet_window_gates_only_within_threshold() {
        assert!(!is_move_recent(0, 500), "从未移动（0）恒为 false");
        assert!(is_move_recent(1_000, 1_200), "静默窗内视为移动中");
        assert!(is_move_recent(1_000, 1_299), "阈值边界前仍算移动中");
        assert!(!is_move_recent(1_000, 1_300), "静默窗过期恢复同步");
        assert!(!is_move_recent(1_000, 999), "时钟回拨按已过期处理");
    }

    #[test]
    fn drag_moved_note_enters_quiet_window() {
        let s = state();
        assert!(!s.is_move_quiet(), "从未移动时不静默");
        s.note_drag_moved();
        assert!(s.is_move_quiet(), "记录移动后应进入静默窗");
    }
}

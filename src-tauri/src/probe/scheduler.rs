//! 探针调度：后台线程按 `config::PROBE_INTERVAL_MS` 轮询（C2.3 / C6 扩展）。
//!
//! 纪律：线程内**禁止 panic**——探针失败静默并下轮重试（桌宠不因探针异常崩溃）；
//! 线程随进程退出（持有 `AppHandle` clone，不做显式 join）。
//!
//! C6 扩展（同线程循环内顺带读取，**不新增轮询线程**）：
//! - idle 翻转状态机：≥`IDLE_THRESHOLD_MS` 由 active→idle 才 emit（省事件洪泛）；
//! - 前台进程变化去重 emit（同名不重复发，节流由 `FOREGROUND_PROBE_EVERY_N` 控频）。

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::cursor;
use super::foreground;
use super::idle;
use super::selection;
use crate::config::{FOREGROUND_PROBE_EVERY_N, IDLE_THRESHOLD_MS, PROBE_INTERVAL_MS};
use crate::state::AppState;

/// `probe:cursor:move` 事件负载（屏幕坐标，camelCase）。
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPayload {
    pub x: i32,
    pub y: i32,
}

/// `probe:idle:changed` 事件负载（camelCase）。`state` 为翻转后的状态。
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdlePayload {
    pub state: IdleState,
    pub idle_ms: u64,
}

/// 空闲状态（camelCase 序列化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IdleState {
    Idle,
    Active,
}

/// `probe:foreground:changed` 事件负载（camelCase）。`process` 为进程名或 null。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForegroundPayload {
    pub process: Option<String>,
}

/// 纯函数：idle 翻转判定（仅翻转才返回 `Some`，省事件洪泛）。
///
/// - active → idle（时长达阈值）：返回 `Idle`；
/// - idle → active（时长回到阈值内）：返回 `Active`；
/// - 状态未变：返回 `None`（不 emit）。
pub fn idle_flip(was_idle: bool, idle_ms: u64, threshold_ms: u64) -> Option<IdleState> {
    let is_idle = idle_ms >= threshold_ms;
    if is_idle && !was_idle {
        Some(IdleState::Idle)
    } else if !is_idle && was_idle {
        Some(IdleState::Active)
    } else {
        None
    }
}

/// 启动鼠标坐标轮询线程（全应用唯一轮询源）。
pub fn start_cursor_probe(app: AppHandle) {
    std::thread::spawn(move || {
        let mut tick: u64 = 0;
        loop {
            // 1) 鼠标坐标（C2）：缓存 + 广播 lookAt
            if let Some(state) = app.try_state::<AppState>() {
                match cursor::current_position() {
                    Ok((x, y)) => {
                        if let Ok(mut slot) = state.cursor.lock() {
                            *slot = (x, y);
                        }
                        let _ = app.emit("probe:cursor:move", CursorPayload { x, y });
                    },
                    Err(_) => {
                        // 探针瞬时失败（如无桌面会话）：静默重试，不 panic、不刷日志
                    },
                }

                // 2) 空闲翻转状态机（C6）：每轮廉价的 GetLastInputInfo
                if let Ok(idle_ms) = idle::idle_ms() {
                    // poisoned 时跳过本轮（不翻转状态、不 emit）
                    if let Ok(mut guard) = state.idle_active.lock() {
                        let was_idle = *guard;
                        if let Some(new_state) = idle_flip(was_idle, idle_ms, IDLE_THRESHOLD_MS) {
                            *guard = new_state == IdleState::Idle;
                            let _ = app.emit(
                                "probe:idle:changed",
                                IdlePayload { state: new_state, idle_ms },
                            );
                        }
                    }
                }

                // 3) 前台进程变化去重 emit（C6）：节流查频（OpenProcess 较重）
                tick = tick.wrapping_add(1);
                if tick.is_multiple_of(FOREGROUND_PROBE_EVERY_N) {
                    let name = foreground::foreground_process_name();
                    if let Ok(mut last) = state.last_foreground.lock() {
                        let changed = match (&name, &*last) {
                            (Some(n), Some(l)) => n != l,
                            (None, Some(_)) => true,
                            (Some(_), None) => true,
                            (None, None) => false,
                        };
                        if changed {
                            *last = name.clone();
                            let _ = app.emit(
                                "probe:foreground:changed",
                                ForegroundPayload { process: name },
                            );
                        }
                    }
                }
            }

            // 4) 划词手势：只在同一 100ms 轮询里检查置位并 emit，不新增轮询线程。
            selection::poll_selection(&app);

            std::thread::sleep(Duration::from_millis(PROBE_INTERVAL_MS));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_flip_only_on_transition() {
        // active → idle（达阈值）
        assert_eq!(idle_flip(false, IDLE_THRESHOLD_MS, IDLE_THRESHOLD_MS), Some(IdleState::Idle));
        // idle → active（回到阈值内）
        assert_eq!(idle_flip(true, 1_000, IDLE_THRESHOLD_MS), Some(IdleState::Active));
        // 状态未变：None（不 emit）
        assert_eq!(idle_flip(false, 1_000, IDLE_THRESHOLD_MS), None);
        assert_eq!(idle_flip(true, IDLE_THRESHOLD_MS, IDLE_THRESHOLD_MS), None);
        // 边界：恰好差 1ms 未达
        assert_eq!(idle_flip(false, IDLE_THRESHOLD_MS - 1, IDLE_THRESHOLD_MS), None);
    }
}

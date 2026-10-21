//! 探针调度：后台线程按 `config::PROBE_INTERVAL_MS` 轮询（C2.3）。
//!
//! 纪律：线程内**禁止 panic**——探针失败静默并下轮重试（桌宠不因探针异常崩溃）；
//! 线程随进程退出（持有 `AppHandle` clone，不做显式 join）。

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::cursor;
use crate::config::PROBE_INTERVAL_MS;
use crate::state::AppState;

/// `probe:cursor:move` 事件负载（屏幕坐标，camelCase）。
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPayload {
    pub x: i32,
    pub y: i32,
}

/// 启动鼠标坐标轮询线程（全应用唯一轮询源）。
pub fn start_cursor_probe(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            match cursor::current_position() {
                Ok((x, y)) => {
                    // 1) 进程内缓存（供穿透状态机读取，零 IPC）
                    if let Some(state) = app.try_state::<AppState>()
                        && let Ok(mut slot) = state.cursor.lock()
                    {
                        *slot = (x, y);
                    }
                    // 2) 事件广播（供窗口 A 的 lookAt 消费）
                    let _ = app.emit("probe:cursor:move", CursorPayload { x, y });
                },
                Err(_) => {
                    // 探针瞬时失败（如无桌面会话）：静默重试，不 panic、不刷日志
                },
            }
            std::thread::sleep(Duration::from_millis(PROBE_INTERVAL_MS));
        }
    });
}

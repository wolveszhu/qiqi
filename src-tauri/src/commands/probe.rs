//! probe 命令域（薄层）：C2 只暴露 cursor 快照；idle/foreground 归 C6。
//!
//! 说明：cursor 实时值已由 `probe:cursor:move` 事件广播（窗口 A 的 lookAt 消费），
//! 本命令供 D1/D2 等**非订阅场景**读取最近一次坐标（读缓存，零轮询）。

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

/// 读取最近一次 cursor 坐标（屏幕坐标；未轮询到则为 0,0）。
#[tauri::command]
pub fn probe_get_cursor(state: State<'_, AppState>) -> Result<(i32, i32), AppError> {
    let slot = state.cursor.lock().map_err(|e| AppError::Db(format!("获取状态锁失败: {e}")))?;
    Ok(*slot)
}

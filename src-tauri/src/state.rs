//! 全局状态容器：命令层经 `tauri::State` 访问（规范 §4）。
//!
//! TODO(B3): 正式化为 `pub struct AppState { pub db: Mutex<Connection> }`
//! TODO(B4): db 字段由 connection::init 注入（setup 内 app.manage）
//! 骨架期保持占位：无字段，避免未接线的死代码告警。

/// 应用级状态占位（B3/B4 落位）。
pub struct AppState;

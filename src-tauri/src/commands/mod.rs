//! IPC 命令薄层（规范 §4：`#[tauri::command]` 只做参数校验 + 转发领域模块）。
//!
//! 红线（code-standards §5 / 规范 §4.1）：
//! - 本目录是 `#[tauri::command]` 的**唯一**合法栖息地（grep 走查项）；
//! - 禁止业务规则、禁止直接拼 SQL（参数化在 repositories）、禁止 emit 事件（事件在领域模块）；
//! - 错误一律 `AppError` + `?` 传播，不 unwrap。
//!
//! 已实装：window（B1 临时通道，C2 扩展）、settings/pet/chat/memory（B5）；
//! TODO(C2/C6): probe.rs 命令（探针快照、proactive_generate）落位。

pub mod chat;
pub mod memory;
pub mod pet;
pub mod update;
pub mod probe;
pub mod quota;
pub mod search_settings;
pub mod settings;
pub mod translate;
pub mod window;

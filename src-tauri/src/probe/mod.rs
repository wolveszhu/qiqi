//! 探针域：系统状态读取（规范 §4.1）。
//!
//! 硬约束（探针单点原则）：`GetCursorPos` 全应用只允许 cursor.rs 一处；
//! 穿透判断读进程内缓存、lookAt 走事件，杜绝双轮询。
//!
//! 唯一写入例外：`selection.rs` 在用户**主动点击**划词图标后改写剪贴板取词，
//! 并在读完立即恢复原内容（详见该文件文档），手势阶段绝不写入。
//!
//! C2 实装：cursor（坐标）+ scheduler（轮询线程）。
//! TODO(C6): idle.rs（GetLastInputInfo）、foreground.rs（前台进程名）。

pub mod cursor;
pub mod foreground;
pub mod idle;
pub mod scheduler;
pub mod selection;

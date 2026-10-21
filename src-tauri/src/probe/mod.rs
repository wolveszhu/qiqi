//! 探针域：系统状态读取（规范 §4.1）。**只读系统，禁止写入**。
//!
//! 硬约束（探针单点原则）：`GetCursorPos` 全应用只允许 cursor.rs 一处；
//! 穿透判断读进程内缓存、lookAt 走事件，杜绝双轮询。
//!
//! C2 实装：cursor（坐标）+ scheduler（轮询线程）。
//! TODO(C6): idle.rs（GetLastInputInfo）、foreground.rs（前台进程名）。

pub mod cursor;
pub mod scheduler;

//! 探针域：系统状态读取（规范 §4.1）。
//!
//! 硬约束（探针单点原则）：`GetCursorPos` 全应用只允许 cursor.rs 一处；
//! 穿透判断读进程内缓存、lookAt 走事件，杜绝双轮询。
//!
//! 剪贴板边界（2026-11-16 零污染改造）：`selection.rs` 的手势探测与取词一律走
//! `uia.rs` 的只读 TextPattern，**不读写剪贴板、不注入按键**；唯一写入剪贴板的
//! 入口是用户主动点击「复制」/「复制译文」（`write_clipboard_text`）。
//!
//! C2 实装：cursor（坐标）+ scheduler（轮询线程）。
//! TODO(C6): idle.rs（GetLastInputInfo）、foreground.rs（前台进程名）。

pub mod cursor;
pub mod foreground;
pub mod idle;
pub mod scheduler;
pub mod selection;
pub mod uia;

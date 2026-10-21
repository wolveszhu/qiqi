//! Win32 探针域（规范 §4.1：**探针单点原则**）。
//!
//! 硬约束：`GetCursorPos` 全应用只允许 `probe/cursor.rs` 一处轮询（100ms），
//! 穿透切换与 lookAt 共用同一数据源——CPU < 2% 红线的结构保障，禁止新增第二轮询。
//!
//! TODO(C2): `cursor.rs`（轮询 + 状态翻转 emit + 进程内缓存双通道）、`click_through.rs`（命中判定纯函数）；
//! TODO(C6): `idle.rs`（GetLastInputInfo 30min 翻转）、`foreground.rs`（仅取进程名，禁读窗口内容）。

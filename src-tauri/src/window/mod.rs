//! 窗口域：窗口 A/B 句柄管理、拖拽、穿透、托盘、快捷键（规范 §4.1）。
//!
//! TODO(C2): `drag.rs`（系统接管拖拽）、`click_through.rs`（穿透状态机，
//! 仅翻转调用系统 API）、`tray.rs`、`shortcut.rs`；
//! TODO(B1): 窗口 B CloseRequested → prevent_close + hide（接线在 lib.rs）。

pub mod manager;

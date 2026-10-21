//! 窗口域：窗口 A/B 句柄管理、拖拽、点击穿透、托盘（规范 §4.1）。
//!
//! C2 已实装：manager（label 常量 + 显隐）、drag（系统接管拖拽）、
//! click_through（命中判定 + 翻转驱动）、tray（托盘菜单）。
//! TODO(C2.7): shortcut.rs（快捷键处理）落位后由 lib.rs 统一装配。

pub mod click_through;
pub mod drag;
pub mod manager;
pub mod tray;

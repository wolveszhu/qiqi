//! 全局状态容器：命令层经 `tauri::State` 访问（规范 §4）。
//!
//! TODO(B4): `db` 连接由 `db::connection::init` 注入（setup 内 `app.manage`）；
//! TODO(C2): cursor/hit_region 缓存字段（穿透状态机）——按任务落位，不提前加。

use std::sync::Mutex;

use rusqlite::Connection;

/// 应用级状态：经 `app.manage(AppState)` 注册，命令层经 `State<AppState>` 访问。
///
/// `db` 字段在 B4 接线前无消费方——pub 结构 + pub 字段属 lib crate 公开 API，
/// 不触发 dead_code（B3 决策：定义先行，注入随后，见 B3 子计划风险表）。
pub struct AppState {
    /// SQLCipher 连接（B4 注入；C2 穿透状态机的 cursor/hit_region 缓存预留其后）。
    pub db: Mutex<Connection>,
}

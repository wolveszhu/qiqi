//! 数据层域：SQLCipher 连接 + 迁移链 + 模型 + 仓储（规范 §4.1）。
//!
//! 表清单、各文件职责与「新增/修改数据表操作路径」见同目录 `README.md`。
//!
//! - `connection.rs`：加密连接、密钥、vec 扩展注册、版本三分支与迁移前备份；
//! - `migrations.rs` + `migrations/`：有序迁移链与各版本 DDL；
//! - `models/`：行结构体 / value JSON 结构 / 命令 DTO（重导出，路径固定）；
//! - `repositories/`：按表分文件，只写参数化 SQL，不做业务决策。

pub mod connection;
pub mod migrations;
pub mod models;
pub mod repositories;

pub use connection::{Db, init};

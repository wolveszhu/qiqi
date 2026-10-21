//! 数据层域：SQLCipher 连接 + 迁移链 + 仓储（规范 §4.1）。
//!
//! - `models.rs`：行结构体与命令 DTO（serde camelCase，与 `src/api/types.ts` 对齐）；
//! - `repositories/`：按表分文件，只写参数化 SQL，不做业务决策。

pub mod connection;
pub mod migrations;
pub mod models;
pub mod repositories;

pub use connection::{Db, init};

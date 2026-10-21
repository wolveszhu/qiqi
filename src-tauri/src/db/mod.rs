//! 数据层域：SQLCipher 连接 + 迁移链 + 仓储（规范 §4.1）。
//!
//! TODO(B5):
//! - `models.rs`：行结构体（serde，与 `src/api/types.ts` 对齐）；
//! - `repositories/`：按表分文件，只写参数化 SQL，不做业务决策。

pub mod connection;
pub mod migrations;

pub use connection::{Db, init};

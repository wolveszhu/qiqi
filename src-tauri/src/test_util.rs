//! 测试夹具（仅 `#[cfg(test)]`）：临时加密库构造。
//!
//! 复用 B4 的 `init_with_key`（固定密钥旁路 keyring），每用例独立临时目录，
//! 与 `tests/*.rs` 的 fixture 口径一致；供 `commands/` 命令 inner 函数**直调用**
//! （F1.2：命令薄壳锁库后转发 `inner(&conn)`，测试直接验证 inner，无需 `State`/runtime）。

use rusqlite::Connection;
use tempfile::TempDir;

use crate::db::connection::{Db, init_with_key, register_vec_extension};

/// 固定测试密钥（64 hex，同生产 32 字节 hex 形态）。
const TEST_KEY: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

/// 新建临时库并完成全链迁移（返回目录句柄 + 连接；目录句柄需保持存活）。
pub fn fresh_db() -> (TempDir, Connection) {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");
    let Db { conn } = init_with_key(&db_path, TEST_KEY).expect("初始化");
    let conn = conn.into_inner().expect("取连接");
    (dir, conn)
}

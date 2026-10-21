//! 数据层域：SQLCipher 连接 + 迁移链 + 仓储（规范 §4.1）。
//!
//! TODO(B4):
//! - `connection.rs`：凭据管理器取/建 key → sqlite-vec auto-extension → `PRAGMA key` → 迁移；
//! - `migrations/`：M1 核心表+种子 → M2 vec0 虚拟表（版本以 rusqlite_migration API 管理）；
//! - `models.rs`：行结构体（serde，与 `src/api/types.ts` 对齐）；
//! - `repositories/`：按表分文件，只写参数化 SQL，不做业务决策。

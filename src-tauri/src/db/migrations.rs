//! 有序迁移链：M1 核心表+种子 → M2 vec0 虚拟表（规范 §4 / rule.md §5）。
//!
//! 铁律：任何结构变更都**追加新迁移**，禁止改历史 SQL；每条迁移独立事务、失败回滚；
//! 版本读取以 rusqlite_migration 内部 `PRAGMA user_version` 为准，禁止手工覆写（附录 A §A.6）。
//!
//! API 实态核对（rusqlite_migration 1.3.1，B4 实施时查证）：
//! - 迁移构造 = `M::up(sql)`（无 `Migration::from_sql`）；
//! - `current_version(&Connection) -> SchemaVersion`（NoneSet/Inside/Outside）；
//! - `to_latest(&mut Connection)` / `to_version(&mut Connection, version)`；
//! - 链长度无 `len()`——目标版本以 `Migrations` 内元素数由本模块自行导出常量。

use rusqlite_migration::{M, Migrations};

use crate::config::EMBEDDING_DIM;

/// 应用支持的目标 schema 版本（= 链上迁移条数）。
/// 新迁移追加时同步 +1；`connection::apply_migrations` 据此判定版本三分支。
pub const TARGET_VERSION: usize = 3;

/// 应用支持的迁移链（M1 核心表+种子 → M2 vec0 → M3 tiered 数据迁移）。
///
/// M2 的 SQL 由 `EMBEDDING_DIM` 运行时渲染，而 `M::up` 只接受 `&'static str`——
/// 以 `Box::leak` 转为 `'static`（进程内迁移链仅构造一次，泄漏量 = 一条 SQL，可接受；
/// 已评估备选：`LazyLock` 缓存同样需泄漏，无更优解，登记于此）。
pub fn chain() -> Migrations<'static> {
    let m2_sql: &'static str = Box::leak(
        format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS vec_memory USING vec0 (\
                 embedding float[{EMBEDDING_DIM}], \
                 +source_text TEXT \
             );"
        )
        .into_boxed_str(),
    );
    Migrations::new(vec![
        // M1：7 张普通表 + 4 索引 + 幂等种子数据（SQL 静态文件）
        M::up(include_str!("migrations/M1__init_core.sql")),
        // M2：vec0 虚拟表；维度必须与 config::EMBEDDING_DIM 单点一致（勘误#6）——
        // 由代码渲染而非 SQL 文件，杜绝两处维度定义漂移。
        M::up(m2_sql),
        // M3：勘误 #7 数据迁移（app_settings 扁平结构 → 三档 tiered）；
        // 仅影响勘误 #7 前创建的旧库，新库种子已正确（迁移内幂等）。
        M::up(include_str!("migrations/M3__llm_tiers.sql")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_migrates_empty_db_to_target_version() {
        // M::up(String) 渲染正确 + 全链可执行：空库 → to_latest → 版本=TARGET_VERSION
        crate::db::connection::register_vec_extension().expect("vec 注册");
        let mut conn = rusqlite::Connection::open_in_memory().expect("内存库");
        chain().to_latest(&mut conn).expect("迁移");
        let current: usize = chain().current_version(&conn).expect("版本读取").into();
        assert_eq!(current, TARGET_VERSION);
    }

    #[test]
    fn m2_sql_renders_dimension_from_config() {
        // M2 是链上第 2 条；验证 vec_memory 的维度由 config 单点渲染（勘误#6）
        crate::db::connection::register_vec_extension().expect("vec 注册");
        let mut conn = rusqlite::Connection::open_in_memory().expect("内存库");
        chain().to_latest(&mut conn).expect("迁移");
        let sql: String = conn
            .query_row("SELECT sql FROM sqlite_master WHERE name = 'vec_memory'", [], |row| {
                row.get(0)
            })
            .expect("vec_memory 存在");
        assert!(sql.contains(&format!("float[{EMBEDDING_DIM}]")), "维度应由 config 渲染: {sql}");
    }
}

//! 有序迁移链 M1–M12（DDL / 数据迁移 / 种子；规范 §4 / rule.md §5）。
//!
//! 链全表与改表操作路径见同目录 `README.md` §3–§4。
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
/// 新迁移追加时同步 +1（操作路径见 `README.md` §4.1）；
/// `connection::apply_migrations` 据此判定版本三分支。
pub const TARGET_VERSION: usize = 12;

/// 应用支持的迁移链（M1 核心表+种子 → M2 vec0 → M3 tiered → M4 dnd/频率 →
/// M5 自定义模型表 → M6 内置模型表 → M7 清空三档 → M8 额度用量 →
/// M9 搜索配置 → M10 额度平台维度与存量回填 → M11 划词翻译配置表 →
/// M12 内置翻译凭据种子 + 用户自填多服务商表）。
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
        // M4：勘误 #13 后数据迁移（type=2 旧单段勿扰 → 多段 + frequency 新结构）；
        // 修正 M1 种子遗留的旧格式（新库 frequency 持久化空串），迁移内幂等。
        M::up(include_str!("migrations/M4__dnd_tiers.sql")),
        // M5：custom_model_settings 自定义大模型表（与 default_model_settings 对称，
        // 多 sort_order 供前端排序；无种子，用户自配）。
        M::up(include_str!("migrations/M5__custom_models.sql")),
        // M6：default_model_settings 内置免费额度模型（智谱/硅基流动种子）。
        // 原加在 M1，但迁移按版本号执行不校验内容，存量库不会重跑 M1，故独立成迁移；
        // 顺带把额度默认口径切到 BYOK（is_custom_quota_used=2），与 M1 DDL 默认值意图对齐。
        M::up(include_str!("migrations/M6__default_models.sql")),
        // M7：清空三档配置（用户决策：三档 UI 合并为平台列表）。
        // 只重置 value、保留行与 is_custom_quota_used；幂等（同值 UPDATE）。
        M::up(include_str!("migrations/M7__clear_llm_tiers.sql")),
        // M8：内置体验额度用量明细（quota_usage）+ 月度汇总缓存（quota_usage_monthly）。
        // 只记内置段（tier='builtin'）；自定义 BYOK 不落库、不展示。
        M::up(include_str!("migrations/M8__quota_usage.sql")),
        // M9：联网搜索服务配置（search_settings，单一行 id=1，BYOK）。
        // provider/api_key/enabled/max_results/lang；Key 仅库内流转，接口不回明文。
        M::up(include_str!("migrations/M9__search_settings.sql")),
        // M10：额度用量加 platform 维度 + 平台月表 + 存量回填 + 月度额度改 100,000。
        // 仅加量（ADD COLUMN / CREATE TABLE）与幂等回填，不改 M8 历史 SQL。
        M::up(include_str!("migrations/M10__quota_platform.sql")),
        // M11：划词翻译配置（单行 id=1），只建表、不写种子。
        M::up(include_str!("migrations/M11__translate_settings.sql")),
        // M12：已登记的生产凭据（内置百度 APPID/密钥）幂等种子 + custom_translate_providers。
        // 存量库不会重跑 M11，故凭据与新表一律追加本迁移；构建期环境变量可覆盖内置凭据。
        M::up(include_str!("migrations/M12__translate_providers.sql")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m11_only_creates_table_then_m12_seeds_builtin() {
        crate::db::connection::register_vec_extension().expect("vec 注册");
        let mut conn = rusqlite::Connection::open_in_memory().expect("内存库");
        chain().to_version(&mut conn, 11).expect("迁移到 M11");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM translate_settings", [], |row| row.get(0))
            .expect("表应存在");
        assert_eq!(count, 0, "M11 只建表，不写种子");

        chain().to_latest(&mut conn).expect("迁移到 M12");
        let current: usize = chain().current_version(&conn).expect("版本读取").into();
        assert_eq!(current, TARGET_VERSION);
        let (count, app_id, secret): (i64, String, String) = conn
            .query_row(
                "SELECT COUNT(*), builtin_app_id, builtin_secret FROM translate_settings WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("读内置凭据");
        assert_eq!(count, 1, "M12 应播种单行配置");
        assert!(!app_id.is_empty() && !secret.is_empty(), "内置凭据不得为空");

        chain().to_latest(&mut conn).expect("重复迁移");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM translate_settings", [], |row| row.get(0))
            .expect("统计行数");
        assert_eq!(count, 1, "幂等：重复迁移不新增行");
    }

    #[test]
    fn m12_upgrades_existing_builtin_row_without_overwriting_user_config() {
        crate::db::connection::register_vec_extension().expect("vec 注册");
        let mut conn = rusqlite::Connection::open_in_memory().expect("内存库");
        chain().to_version(&mut conn, 11).expect("迁移到 M11");
        conn.execute(
            "INSERT INTO translate_settings (id, enabled, target_lang, provider, custom_api_key) \
             VALUES (1, 1, 'en', 'deepl', 'user-key')",
            [],
        )
        .expect("模拟存量空凭据行");

        chain().to_latest(&mut conn).expect("迁移到 M12");
        let (target, provider, key, builtin_id): (String, String, String, String) = conn
            .query_row(
                "SELECT target_lang, provider, custom_api_key, builtin_app_id \
                 FROM translate_settings WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("读配置");
        assert_eq!(target, "en", "不得覆盖用户译入语言");
        assert_eq!(provider, "deepl", "不得覆盖用户服务商");
        assert_eq!(key, "user-key", "不得覆盖用户自定义 Key");
        assert!(!builtin_id.is_empty(), "存量行应补齐内置凭据");

        let providers: i64 = conn
            .query_row("SELECT COUNT(*) FROM custom_translate_providers", [], |row| row.get(0))
            .expect("多服务商表应存在");
        assert_eq!(providers, 0, "用户自配表首发不得预置数据");
    }

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
    fn m6_upgrades_existing_db_and_seeds_builtin_models() {
        // 存量库场景：先跑到 M5（旧 TARGET），再 to_latest → M6 生效且种子写入
        crate::db::connection::register_vec_extension().expect("vec 注册");
        let mut conn = rusqlite::Connection::open_in_memory().expect("内存库");
        chain().to_version(&mut conn, 5).expect("迁移到 M5");
        chain().to_latest(&mut conn).expect("迁移到 M6");
        let current: usize = chain().current_version(&conn).expect("版本读取").into();
        assert_eq!(current, TARGET_VERSION);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM default_model_settings", [], |row| row.get(0))
            .expect("表应存在");
        assert_eq!(count, 2, "智谱 + 硅基流动种子应写入");

        // 幂等：重复 to_latest 不重复插入
        chain().to_latest(&mut conn).expect("重复迁移");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM default_model_settings", [], |row| row.get(0))
            .expect("表应存在");
        assert_eq!(count, 2, "幂等：不重复插入");
    }

    #[test]
    fn m7_clears_tiers_but_keeps_row_and_quota_mode() {
        // 存量库场景：M6 版本 + 已配三档 + 额度模式 → M7 后三档全空、行与额度模式保留
        crate::db::connection::register_vec_extension().expect("vec 注册");
        let mut conn = rusqlite::Connection::open_in_memory().expect("内存库");
        chain().to_version(&mut conn, 6).expect("迁移到 M6");
        conn.execute(
            "UPDATE app_settings SET value = '{\"main\":{\"baseUrl\":\"https://old/v1\",\"model\":\"m\",\"apiKey\":\"sk-old\",\"modelCustom\":\"\"}}', is_custom_quota_used = 2 WHERE type = 1",
            [],
        )
        .expect("写入存量三档");

        chain().to_latest(&mut conn).expect("迁移到 M7");
        let current: usize = chain().current_version(&conn).expect("版本读取").into();
        assert_eq!(current, TARGET_VERSION);

        let tiered = crate::db::repositories::app_settings::get_tiered(&conn).expect("读三档");
        assert!(tiered.main.base_url.is_empty(), "main 应被清空");
        assert!(tiered.main.api_key.is_empty(), "Key 应被清空");
        assert!(tiered.memory.base_url.is_empty() && tiered.fallback.base_url.is_empty());
        let quota = crate::db::repositories::app_settings::get_quota_mode(&conn).expect("读额度");
        assert_eq!(quota, 2, "额度模式应保留（不属于三档配置）");

        // 幂等：重复执行同值 UPDATE 无副作用
        chain().to_latest(&mut conn).expect("重复迁移");
        let tiered = crate::db::repositories::app_settings::get_tiered(&conn).expect("读三档");
        assert!(tiered.main.base_url.is_empty());
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

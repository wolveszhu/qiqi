//! 发布数据运行时守卫：干净空库跑完完整迁移链后，只应有系统基础数据。
//!
//! 依据 `src/db/README.md` §7.2 的生产基线。安装包不含 `.db` 文件——种子以
//! `include_str!` 编译进二进制、首次启动时写入；因此本测试在**干净空库**上
//! 复现「用户首次启动后得到的库」，是唯一能真实反映发布数据状态的验证位。
//!
//! 任何迁移新增/修改种子导致行数或默认值变化，必须同步更新 `BASELINE_ROWS`
//! 与 README §7.2，否则 CI（`.github/workflows/release.yml`）阻断发布——
//! 这是刻意的「改表必经」闭环，防止测试数据悄悄混入发布包。

// 集成测试 fixture 用 unwrap/expect/index 属合理用法，与其余集成测试口径一致。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use rusqlite::Connection;

use qiqi_lib::db::connection::{init_with_key, register_vec_extension, Db};
use qiqi_lib::db::migrations::TARGET_VERSION;

/// 固定测试密钥（64 hex 字符，同生产 32 字节 hex 形态）。
const TEST_KEY: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

/// 生产基线：表 / COUNT 语句 / 首次启动后允许的行数（见 README §7.2）。
///
/// 表名与 SQL 全部写死为常量，不用 `format!` 拼接——表数量固定且可枚举，
/// 拼接既无必要也会违反「SQL 禁止字符串拼接」红线（rule.md §H）。
const BASELINE_ROWS: &[(&str, &str, i64)] = &[
    // ---- 系统基础数据（幂等种子，允许非 0）----
    ("pet_states", "SELECT COUNT(*) FROM pet_states", 1),
    ("app_settings", "SELECT COUNT(*) FROM app_settings", 2),
    ("default_model_settings", "SELECT COUNT(*) FROM default_model_settings", 2),
    ("search_settings", "SELECT COUNT(*) FROM search_settings", 1),
    // ---- 运行时数据（必须为空）----
    ("messages", "SELECT COUNT(*) FROM messages", 0),
    ("summaries", "SELECT COUNT(*) FROM summaries", 0),
    ("facts", "SELECT COUNT(*) FROM facts", 0),
    ("vec_memory", "SELECT COUNT(*) FROM vec_memory", 0),
    ("affection_logs", "SELECT COUNT(*) FROM affection_logs", 0),
    ("diaries", "SELECT COUNT(*) FROM diaries", 0),
    ("custom_model_settings", "SELECT COUNT(*) FROM custom_model_settings", 0),
    ("quota_usage", "SELECT COUNT(*) FROM quota_usage", 0),
    ("quota_usage_monthly", "SELECT COUNT(*) FROM quota_usage_monthly", 0),
];

/// 发布构建中禁止出现的测试 / 临时数据标记（与 README §7.2 一致）。
const FORBIDDEN_MARKERS: &[&str] = &["sk-old", "old/v1", "TEST_", "test-fixture", "mock-data"];

/// 新建临时库并跑完 M1–M9，等价于用户首次启动。
fn fresh_db() -> (tempfile::TempDir, Connection) {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");
    let Db { conn } = init_with_key(&db_path, TEST_KEY).expect("初始化");
    (dir, conn.into_inner().expect("取连接"))
}

/// 断言一段库内文本不含任何测试 / 临时标记。
fn assert_clean(label: &str, text: &str) {
    for marker in FORBIDDEN_MARKERS {
        assert!(!text.contains(marker), "{label} 含禁止标记 `{marker}`，发布数据不干净: {text}");
    }
}

#[test]
fn clean_install_has_only_baseline_rows() {
    let (_dir, conn) = fresh_db();

    for (table, sql, expected) in BASELINE_ROWS {
        let actual: i64 = conn.query_row(sql, [], |row| row.get(0)).expect("统计行数");
        assert_eq!(
            actual, *expected,
            "`{table}` 行数偏离生产基线（期望 {expected}，实得 {actual}）；\
             若迁移确实改了种子，请同步更新 README §7.2 与 BASELINE_ROWS"
        );
    }
}

#[test]
fn all_baseline_tables_exist_and_schema_version_matches() {
    let (_dir, conn) = fresh_db();

    for (table, _, _) in BASELINE_ROWS {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("查询表");
        assert!(exists, "生产基线表缺失: {table}");
    }

    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).expect("读版本");
    assert_eq!(version, TARGET_VERSION as i64, "首次启动应到达目标 schema 版本");
}

#[test]
fn baseline_values_are_production_defaults() {
    let (_dir, conn) = fresh_db();

    // 桌宠状态：全新出厂态
    let (name, level, points, renderer): (String, i64, i64, String) = conn
        .query_row(
            "SELECT pet_name, relationship_level, affection_points, active_renderer FROM pet_states",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("读 pet_states");
    assert_eq!(name, "喵喵", "桌宠名应为默认值");
    assert_eq!(level, 1, "关系等级应为 Lv1");
    assert_eq!(points, 0, "好感度应为 0（不得带历史互动）");
    assert_eq!(renderer, "code_cat", "形象应为默认猫");

    // type=1：M7 清空三档，不得残留 M1 的开发期默认 URL
    let (tiered, quota_mode): (String, i64) = conn
        .query_row(
            "SELECT value, is_custom_quota_used FROM app_settings WHERE type = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("读 type=1");
    assert!(tiered.contains("\"main\""), "三档结构应存在");
    for stale in ["deepseek", "dashscope", "bigmodel"] {
        assert!(
            !tiered.contains(stale),
            "type=1 残留开发期默认配置 `{stale}`，M7 未生效: {tiered}"
        );
    }
    assert_eq!(quota_mode, 2, "首发应为 BYOK 默认口径");

    // type=2：M4 规范化后的勿扰 + 频率
    let dnd: String = conn
        .query_row("SELECT value FROM app_settings WHERE type = 2", [], |row| row.get(0))
        .expect("读 type=2");
    assert!(dnd.contains("\"dnd\""), "勿扰结构应存在");
    assert!(dnd.contains("\"frequency\":\"normal\""), "频率应为默认 normal");

    // 内置模型：恰好两家，且 Key 为已登记的生产凭据（非空）
    let models: Vec<(String, String)> = conn
        .prepare("SELECT name, api_key FROM default_model_settings ORDER BY id")
        .expect("准备")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("查询")
        .collect::<Result<Vec<_>, _>>()
        .expect("收集");
    assert_eq!(models.len(), 2, "内置模型应为 2 家");
    assert!(models.iter().all(|(name, key)| !name.is_empty() && !key.is_empty()));
    assert!(models.iter().any(|(name, _)| name == "智谱"));
    assert!(models.iter().any(|(name, _)| name == "硅基流动"));

    // 联网搜索：未配置 Key、未启用（用户自填 BYOK）
    let (provider, key, enabled): (String, String, i64) = conn
        .query_row(
            "SELECT provider, api_key, enabled FROM search_settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("读 search_settings");
    assert_eq!(provider, "tavily");
    assert!(key.is_empty(), "首发不得预置搜索 API Key");
    assert_eq!(enabled, 0, "联网搜索默认关闭");
}

#[test]
fn baseline_rows_contain_no_test_markers() {
    let (_dir, conn) = fresh_db();

    // 覆盖所有含文本的基础数据列；运行时表已由行数断言保证为空。
    let mut stmt = conn
        .prepare(
            "SELECT pet_name, active_renderer FROM pet_states
             UNION ALL SELECT CAST(type AS TEXT), value FROM app_settings
             UNION ALL SELECT name, models FROM default_model_settings
             UNION ALL SELECT provider, api_key FROM search_settings",
        )
        .expect("准备");
    let texts: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("查询")
        .collect::<Result<Vec<_>, _>>()
        .expect("收集");

    for (a, b) in texts {
        assert_clean("基础数据列 A", &a);
        assert_clean("基础数据列 B", &b);
    }
}

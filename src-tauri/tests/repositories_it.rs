//! 仓储集成测试（B5.7 / rule.md「先测后集成」）：命令层实装前先验证数据访问。
//!
//! 基础设施：临时目录 + 固定测试密钥 + **真实迁移链**（B4 的 M1/M2），
//! 保证测试 schema 与生产完全一致（防 repositories SQL 与表结构漂移）。

// F1.4：集成测试 fixture 用 unwrap/expect/index 属合理用法，统一豁免四项 clippy lint。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use rusqlite::Connection;

use qiqi_lib::db::connection::{Db, init_with_key, register_vec_extension};
use qiqi_lib::db::migrations::{TARGET_VERSION, chain};
use qiqi_lib::db::repositories::{app_settings, diaries, facts, pet_states};

const TEST_KEY: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

/// 建一个已完成全链迁移的临时库连接。
fn fresh_conn() -> (tempfile::TempDir, Connection) {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");
    let Db { conn } = init_with_key(&db_path, TEST_KEY).expect("初始化");
    (dir, conn.into_inner().expect("取连接"))
}

// ---------- 用例 1：app_settings 读写 + JSON 解析 ----------

#[test]
fn case1_app_settings_upsert_and_read() {
    let (_dir, conn) = fresh_conn();

    // 种子行存在（type=1 与 type=2）
    assert!(app_settings::get_row(&conn, 1).expect("查询").is_some());
    assert!(app_settings::get_row(&conn, 2).expect("查询").is_some());
    assert!(app_settings::get_row(&conn, 99).expect("查询").is_none());

    // upsert：已存在 → UPDATE（不新增行）
    app_settings::upsert_value(&conn, 1, r#"{"main":{"baseUrl":"https://x/v1"}}"#).expect("upsert");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_settings WHERE type = 1", [], |r| r.get(0))
        .expect("计数");
    assert_eq!(count, 1, "UPDATE 不应新增行");

    // upsert：不存在 → INSERT
    app_settings::upsert_value(&conn, 3, "{}").expect("insert");
    assert!(app_settings::get_row(&conn, 3).expect("查询").is_some());

    // 写入的 JSON 可被解析回原值
    let value =
        app_settings::get_row(&conn, 1).expect("查询").and_then(|r| r.value).expect("value");
    let parsed: serde_json::Value = serde_json::from_str(&value).expect("JSON 解析");
    assert_eq!(parsed["main"]["baseUrl"], "https://x/v1");
}

// ---------- 用例 2：额度模式读写 ----------

#[test]
fn case2_quota_mode_roundtrip() {
    let (_dir, conn) = fresh_conn();
    assert_eq!(app_settings::get_quota_mode(&conn).expect("读取"), 1, "缺省应为内置额度");
    app_settings::set_quota_mode(&conn, 2).expect("设置为 BYOK");
    assert_eq!(app_settings::get_quota_mode(&conn).expect("读取"), 2);
}

// ---------- 用例 3：pet_states 单行 upsert 幂等 ----------

#[test]
fn case3_pet_states_single_row_and_update() {
    let (_dir, conn) = fresh_conn();

    // 种子保证仅一行；重复 get 不新增
    let row = pet_states::get(&conn).expect("读取");
    assert_eq!(row.id, 1);
    assert_eq!(row.pet_name, "喵喵");
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM pet_states", [], |r| r.get(0)).expect("计数");
    assert_eq!(count, 1);

    // 更新字段（仅传入项生效）
    pet_states::update_profile(&conn, Some("小七"), Some("code_cat"), None).expect("更新");
    let row = pet_states::get(&conn).expect("读取");
    assert_eq!(row.pet_name, "小七");
    assert_eq!(row.active_renderer, "code_cat");
    // renderer_config 未传 → 保持原值（NULL）
    assert!(row.renderer_config.is_none());

    pet_states::update_profile(&conn, None, None, Some(r#"{"color":"milk"}"#)).expect("更新配置");
    let row = pet_states::get(&conn).expect("读取");
    assert_eq!(row.renderer_config.as_deref(), Some(r#"{"color":"milk"}"#));
    assert_eq!(row.pet_name, "小七", "未传字段不应被覆盖");
}

// ---------- 用例 4：facts / diaries 列表与 limit ----------

#[test]
fn case4_facts_and_diaries_list() {
    let (_dir, conn) = fresh_conn();

    // 种子后为空集
    assert!(facts::list(&conn, 200).expect("查询").is_empty());
    assert!(diaries::list(&conn, 200).expect("查询").is_empty());

    for i in 0..3 {
        conn.execute(
            "INSERT INTO facts (category, content) VALUES (?1, ?2)",
            rusqlite::params![i % 3, format!("事实{i}")],
        )
        .expect("插入 fact");
    }
    conn.execute(
        "INSERT INTO diaries (diary_date, content, mood) VALUES ('2026-10-21', '日记', 2)",
        [],
    )
    .expect("插入 diary");

    let facts_list = facts::list(&conn, 200).expect("查询");
    assert_eq!(facts_list.len(), 3);
    assert_eq!(facts_list[0].content, "事实2", "应按创建时间倒序");

    assert_eq!(facts::list(&conn, 2).expect("查询").len(), 2, "limit 生效");
    assert_eq!(diaries::list(&conn, 200).expect("查询").len(), 1);
}

// ---------- 用例 5：参数化（拼接注入负样例） ----------

#[test]
fn case5_parameterized_queries_reject_injection() {
    let (_dir, conn) = fresh_conn();

    // 恶意字符串经参数传入：应作为**普通值**落库，不产生额外行/不报错
    let evil = "x'); DROP TABLE facts; --";
    pet_states::update_profile(&conn, Some(evil), None, None).expect("参数化写入");
    let row = pet_states::get(&conn).expect("读取");
    assert_eq!(row.pet_name, evil, "参数化应原样保存特殊字符");

    // facts 表仍存在且可查（DROP TABLE 未生效）
    let ok: i64 =
        conn.query_row("SELECT COUNT(*) FROM facts", [], |r| r.get(0)).expect("facts 表仍存在");
    assert_eq!(ok, 0, "注入未生效且表完好");
}

// ---------- 用例 6：迁移链与仓储口径一致（schema 漂移防护） ----------

#[test]
fn case6_schema_matches_migration_chain() {
    let (_dir, conn) = fresh_conn();
    // 仓储依赖的列必须全部存在于真实迁移后的 schema
    for col in [
        ("app_settings", "is_custom_quota_used"),
        ("pet_states", "active_renderer"),
        ("pet_states", "renderer_config"),
        ("facts", "confidence"),
        ("diaries", "mood"),
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info(?1) WHERE name = ?2",
                rusqlite::params![col.0, col.1],
                |r| r.get(0),
            )
            .expect("列检查");
        assert!(exists, "列缺失: {}.{}", col.0, col.1);
    }
    let version: usize = chain().current_version(&conn).expect("版本").into();
    assert_eq!(version, TARGET_VERSION, "M1–M4（含勘误 #7 tiered / 勘误 #13 dnd 迁移）应已注册");
}

// ---------- 用例 7（C3）：tiered 读写 / BYOK 校验 / 双格式兼容 ----------

#[test]
fn case7a_tiered_read_write_and_quota_roundtrip() {
    let (_dir, conn) = fresh_conn();

    // 种子即三档：main 默认 DeepSeek（勘误 #7 后的 M1 种子）
    let main = app_settings::get_tier(&conn, "main").expect("读 main");
    assert_eq!(main.base_url, "https://api.deepseek.com/v1");

    // 保存 memory 档 → 读回一致；main 档不受影响
    let mem = crate_tier("https://dashscope.aliyuncs.com/compatible-mode/v1", "qwen-flash", "");
    app_settings::save_tier(&conn, "memory", &mem).expect("保存 memory");
    let back = app_settings::get_tier(&conn, "memory").expect("读 memory");
    assert_eq!(back.model, "qwen-flash");
    assert_eq!(
        app_settings::get_tier(&conn, "main").expect("读 main").base_url,
        "https://api.deepseek.com/v1",
        "单档保存不应影响其他档"
    );

    // 额度模式往返
    assert_eq!(app_settings::get_quota_mode(&conn).expect("读"), 1);
    app_settings::set_quota_mode(&conn, 2).expect("设 BYOK");
    assert_eq!(app_settings::get_quota_mode(&conn).expect("读"), 2);
}

#[test]
fn case7b_byok_requires_main_key() {
    let (_dir, conn) = fresh_conn();
    // main 无 Key（种子为空）→ BYOK 前置校验失败
    assert!(!app_settings::main_tier_has_key(&conn).expect("校验"), "种子无 Key");
    // 配上 main Key 后校验通过
    let with_key = crate_tier("https://api.deepseek.com/v1", "deepseek-chat", "sk-main");
    app_settings::save_tier(&conn, "main", &with_key).expect("保存 main");
    assert!(app_settings::main_tier_has_key(&conn).expect("校验"));
}

#[test]
fn case7c_unknown_tier_rejected() {
    let (_dir, conn) = fresh_conn();
    assert!(app_settings::get_tier(&conn, "evil").is_err(), "未知档位应报错");
    assert!(app_settings::save_tier(&conn, "evil", &crate_tier("", "", "")).is_err());
}

#[test]
fn case7d_legacy_flat_value_still_readable() {
    let (_dir, conn) = fresh_conn();
    // 构造勘误 #7 前的旧库（M3 之前把 value 改回扁平 → 直接覆盖为扁平模拟旧数据）
    conn.execute(
        "UPDATE app_settings SET value = '{\"llm_base_url\":\"https://api.deepseek.com/v1\",\"llm_api_key\":\"sk-old\",\"llm_model\":\"deepseek-chat\",\"llm_model_custom\":\"\"}' WHERE type = 1",
        [],
    )
    .expect("写回扁平");
    // 双格式兼容：扁平整体视为 main 档
    let main = app_settings::get_tier(&conn, "main").expect("兼容读取");
    assert_eq!(main.base_url, "https://api.deepseek.com/v1");
    assert_eq!(main.api_key, "sk-old");
    assert!(app_settings::get_tier(&conn, "memory").expect("读").model.is_empty());
}

/// 构造单档配置（测试辅助）。
fn crate_tier(base_url: &str, model: &str, api_key: &str) -> qiqi_lib::db::models::LlmTierValue {
    qiqi_lib::db::models::LlmTierValue {
        base_url: base_url.into(),
        model: model.into(),
        api_key: api_key.into(),
        model_custom: String::new(),
    }
}

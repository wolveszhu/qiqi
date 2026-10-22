//! C5 集成测试（§五）：结算事务性 + 跨天衰减模拟（经 DB 核心，无需 AppHandle）。
//!
//! 复用 B4 测试夹具（`temp_db` 初始化 + 迁移 + 建表）。

// F1.4：集成测试 fixture 用 unwrap/expect/index 属合理用法，统一豁免四项 clippy lint。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use rusqlite::Connection;

use qiqi_lib::affection::decay;
use qiqi_lib::affection::level::{level_floor, level_of};
use qiqi_lib::affection::rules::AffectionSignal;
use qiqi_lib::affection::{Mood, settle_decay_core, settle_signal_core};
use qiqi_lib::db::connection::{Db, init_with_key, register_vec_extension};
use qiqi_lib::db::repositories::{affection_logs, pet_states};

/// 固定测试密钥（64 hex，同生产 32 字节 hex 形态）。
const TEST_KEY: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

/// 新建临时库并完成全链迁移。
fn temp_db() -> (tempfile::TempDir, Connection) {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");
    let Db { conn } = init_with_key(&db_path, TEST_KEY).expect("初始化");
    (dir, conn.into_inner().expect("取连接"))
}

#[test]
fn settle_is_transactional_logs_and_states_synced() {
    let (_dir, conn) = temp_db();
    // 初始 0 分 Lv1
    let row = pet_states::get(&conn).unwrap();
    assert_eq!(row.affection_points, 0);

    // 打卡 +10 → 10 分 Lv1，日志 1 条
    let out = settle_signal_core(&conn, AffectionSignal::CheckIn).unwrap();
    assert_eq!(out.delta, 10);
    assert_eq!(out.new_points, 10);
    assert_eq!(out.mood, Mood::Happy);
    let logs1 = affection_logs::recent(&conn, 10).unwrap();
    assert_eq!(logs1.len(), 1, "打卡后流水应为 1 条");

    // 当日重复打卡 → 0（唯一性），日志不再增长
    let out2 = settle_signal_core(&conn, AffectionSignal::CheckIn).unwrap();
    assert_eq!(out2.delta, 0);
    assert_eq!(affection_logs::recent(&conn, 10).unwrap().len(), 1);

    // 抚摸 5 次 → +5，日志累计到 6 条
    for _ in 0..5 {
        settle_signal_core(&conn, AffectionSignal::PetStroke).unwrap();
    }
    let row = pet_states::get(&conn).unwrap();
    assert_eq!(row.affection_points, 15);
    assert_eq!(affection_logs::recent(&conn, 50).unwrap().len(), 6);
}

#[test]
fn settle_clamps_to_ceiling_and_floor_keeps_level() {
    let (_dir, conn) = temp_db();
    // 推到 199（Lv1 顶）→ 再 +10 应封顶 1000 且跨到 Lv4
    // 直接以多次抚摸逼近：用 DeepChat +5 循环
    for _ in 0..200 {
        let _ = settle_signal_core(&conn, AffectionSignal::DeepChat);
    }
    let row = pet_states::get(&conn).unwrap();
    assert_eq!(row.affection_points, 1000, "封顶 1000");
    assert_eq!(row.relationship_level, 4, "满级 Lv4");
}

#[test]
fn cross_day_decay_simulated_via_overridden_dates() {
    let (_dir, conn) = temp_db();
    // 写入 500 分 Lv2，并模拟 last_interaction 在 100 天前、last_decay_at 空
    // 通过 pet_states 直接设定（测试用）
    pet_states::set_affection(&conn, 500, 2).unwrap();
    pet_states::touch_interaction(&conn, "2026-01-01").unwrap();

    // settle_decay_core 使用真实“今天”，与 last_interaction 相差约 100 天
    // 精确天数取决于运行日，但必为净减且停在 Lv2 下限 201（不破档）
    let out = settle_decay_core(&conn).unwrap();
    let out = out.expect("确有衰减");
    assert!(out.delta < 0, "衰减为负");
    let row = pet_states::get(&conn).unwrap();
    assert_eq!(row.affection_points, 201, "止于 Lv2 下限，不破档");
    assert_eq!(row.relationship_level, 2);
    // 衰减流水写入 1 条
    assert_eq!(affection_logs::recent(&conn, 10).unwrap().len(), 1);

    // 幂等：立即再结算（last_decay_at 已更新为今天）→ 无变化
    let again = settle_decay_core(&conn).unwrap();
    assert!(again.is_none(), "同锚点重复结算幂等");
}

#[test]
fn decay_engine_unit_anchor_idempotent() {
    use chrono::NaiveDate;
    let today = NaiveDate::parse_from_str("2026-04-11", "%Y-%m-%d").unwrap();
    let li = NaiveDate::parse_from_str("2026-01-01", "%Y-%m-%d").unwrap();
    // 100 天未互动、Lv2（下限 201）：精确停在 201
    let (np, _a, days) =
        decay::settle_decay(500, level_floor(level_of(500)), Some(li), None, today);
    assert_eq!(days, 97);
    assert_eq!(np, 201);
}

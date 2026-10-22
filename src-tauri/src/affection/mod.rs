//! 好感度域：加分/衰减/等级纯规则引擎（规范 §4.1：零 I/O，可 `#[cfg(test)]` 全测）
//! + 结算编排（事务写库 + emit `affection:changed`）。
//!
//! 子模块：
//! - `rules.rs`：加分规则引擎（打卡/抚摸/深聊/共鸣/禁忌）
//! - `decay.rs`：弹性衰减（>3 天每天 -5 止于档下限，幂等）
//! - `level.rs`：等级映射（0-1000 四档 + 行为指令文案）
//! - `reason.rs`：流水 reason 串常量
//!
//! 编排层（本文件）：
//! - `settle_signal`：命令层调用——单信号事务结算 + emit
//! - `settle_decay_at_startup`：启动/懒结算衰减（幂等，失败仅 log）

pub mod decay;
pub mod level;
pub mod reason;
pub mod rules;

use chrono::{Local, NaiveDate};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use rusqlite::Connection;

use crate::db::repositories::{affection_logs, pet_states};
use crate::error::AppError;

use self::level::Level;
use self::level::{detect_shift, level_index, level_of};
use self::reason::REASON_DECAY;
use self::rules::{AffectionSignal, reason_of};

/// 好感度上限（封顶；与 level.rs 保持一致引用）。
pub use self::level::MAX_POINTS;

/// 情绪基调（窗口 A 只消费 mood；数值仅进窗口 B 看板）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mood {
    Happy,
    Angry,
    Neutral,
}

/// `affection:changed` 事件负载（与前端 `AffectionChangedPayload` 对齐，camelCase）。
///
/// 窗口 A 消费端**只取 `mood`**；`delta/newPoints/newLevel` 仅供窗口 B 看板（D2）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectionChangedDto {
    pub delta: i64,
    pub new_points: i64,
    pub new_level: i64,
    pub mood: Mood,
}

/// 本地今日日期串（`YYYY-MM-DD`，与 SQL `date('now','localtime')` 口径一致）。
fn today_str() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

fn parse_date(s: Option<&str>) -> Option<NaiveDate> {
    s.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}

/// 计算表情基调（升级→happy；降级→angry；同档按 delta 符号）。
fn mood_of(delta: i64, old: Level, new: Level) -> Mood {
    match detect_shift(old, new) {
        level::LevelShift::Up => Mood::Happy,
        level::LevelShift::Down => Mood::Angry,
        level::LevelShift::None => {
            if delta > 0 {
                Mood::Happy
            } else if delta < 0 {
                Mood::Angry
            } else {
                Mood::Neutral
            }
        },
    }
}

/// 单信号结算（DB 核心，无 emit）：事务内更新 pet_states + 写 affection_logs。
///
/// - 分数夹取 0..=MAX_POINTS；等级随分数重算；
/// - 每次结算刷新 `last_interaction`（互动即推后衰减起点）；
/// - 返回事件负载（供调用方 emit / 测试断言）。
///
/// 与 `settle_signal` 分离以便集成测试在无 `AppHandle` 下验证事务性。
pub fn settle_signal_core(
    conn: &Connection,
    signal: AffectionSignal,
) -> Result<AffectionChangedDto, AppError> {
    let row = pet_states::get(conn)?;
    let todays = affection_logs::todays(conn)?;
    // 转领域摘要类型（reason/change_value），与规则引擎解耦仓储结构
    let todays_logs: Vec<rules::TodaysLog> = todays
        .iter()
        .map(|r| rules::TodaysLog { reason: r.reason.clone(), change_value: r.change_value })
        .collect();
    let change = rules::apply(row.affection_points, &signal, &todays_logs);

    let old_level = level_of(row.affection_points);
    let new_points = (row.affection_points + change.delta).clamp(0, MAX_POINTS);
    let new_level = level_of(new_points);

    let today = today_str();
    let tx = conn.unchecked_transaction()?;
    pet_states::set_affection(&tx, new_points, level_index(new_level))?;
    pet_states::touch_interaction(&tx, &today)?;
    // 仅在实际发生变更时写流水（delta=0 的重复打卡/达上限抚摸不再污染 D2 看板）
    if change.delta != 0 {
        affection_logs::insert(&tx, change.delta, reason_of(&signal))?;
    }
    tx.commit()?;

    Ok(AffectionChangedDto {
        delta: change.delta,
        new_points,
        new_level: level_index(new_level),
        mood: mood_of(change.delta, old_level, new_level),
    })
}

/// 单信号结算（命令层入口）：DB 核心 + emit `affection:changed`。
pub fn settle_signal(
    conn: &Connection,
    app: &AppHandle,
    signal: AffectionSignal,
) -> Result<AffectionChangedDto, AppError> {
    let payload = settle_signal_core(conn, signal)?;
    let _ = app.emit("affection:changed", &payload);
    Ok(payload)
}

/// 启动/懒结算衰减（DB 核心，无 emit）：幂等，仅确有衰减天数时写库。
///
/// - 无衰减（days<=0）→ 返回 None，不改库；
/// - 衰减只更新 `last_decay_at` 与分数/等级，**不**动 `last_interaction`。
pub fn settle_decay_core(conn: &Connection) -> Result<Option<AffectionChangedDto>, AppError> {
    let row = pet_states::get(conn)?;
    let today = Local::now().date_naive();
    let li = parse_date(row.last_interaction.as_deref());
    let ld = parse_date(row.last_decay_at.as_deref());

    let floor = level::level_floor(level_of(row.affection_points));
    let (new_points, _anchor, days) =
        decay::settle_decay(row.affection_points, floor, li, ld, today);
    if days <= 0 {
        return Ok(None); // 幂等：无衰减不写库
    }

    let today_s = today.format("%Y-%m-%d").to_string();
    let tx = conn.unchecked_transaction()?;
    pet_states::set_affection(&tx, new_points, level_index(level_of(new_points)))?;
    pet_states::touch_decay(&tx, &today_s)?;
    // 衰减流水（供 D2 看板追溯；change_value 为负）
    affection_logs::insert(&tx, new_points - row.affection_points, REASON_DECAY)?;
    tx.commit()?;

    Ok(Some(AffectionChangedDto {
        delta: new_points - row.affection_points,
        new_points,
        new_level: level_index(level_of(new_points)),
        mood: Mood::Angry, // 衰减恒为负 → 情绪化表达
    }))
}

/// 启动/懒结算衰减（命令层入口）：DB 核心 + emit（有衰减才 emit）。
///
/// 失败仅返回 Err 供调用方 log（不阻断启动）。
pub fn settle_decay_at_startup(app: &AppHandle, conn: &Connection) -> Result<(), AppError> {
    if let Some(payload) = settle_decay_core(conn)? {
        let _ = app.emit("affection:changed", &payload);
    }
    Ok(())
}

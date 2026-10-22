//! affection_logs 表访问（好感度流水；D2 看板数据源）。
//!
//! 红线（rule.md §H）：全部参数化 SQL；错误一律 `AppError::Db`。

use rusqlite::{Connection, params};

use crate::db::models::AffectionLogRow;
use crate::error::AppError;

/// 写入一条流水（change_value 正加负减，reason 为 `affection::reason` 常量）。
pub fn insert(conn: &Connection, change_value: i64, reason: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO affection_logs (change_value, reason) VALUES (?1, ?2)",
        params![change_value, reason],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 当日（本地时区）流水摘要（供规则引擎判定打卡唯一性/抚摸日上限）。
///
/// 与 `affection::settle_signal` 使用的 `today_str()` 同为本地日期口径
/// （SQLite `date('now','localtime')`），跨时区一致。
///
/// ⚠️ `created_at` 默认值为 `CURRENT_TIMESTAMP`（**UTC**），故左值必须先
/// `,'localtime'` 转本地再与本地今日比较——否则 UTC+8 用户在本地 00:00–08:00
/// 两日期相差一天，当日流水漏判 → 打卡唯一性与抚摸日上限失效（F3-F4 回归缺陷，
/// 见 `tests/affection_it.rs::todays_uses_local_date_not_utc` 钉子）。
/// 同口径约定见 `messages::by_date`（注释已明示 UTC 存储 + localtime 过滤）。
pub fn todays(conn: &Connection) -> Result<Vec<AffectionLogRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, change_value, reason, created_at
             FROM affection_logs
             WHERE date(created_at, 'localtime') = date('now','localtime')",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(AffectionLogRow {
                id: row.get(0)?,
                change_value: row.get(1)?,
                reason: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 最近 `limit` 条流水（D2 看板，倒序）。
pub fn recent(conn: &Connection, limit: i64) -> Result<Vec<AffectionLogRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, change_value, reason, created_at
             FROM affection_logs ORDER BY id DESC LIMIT ?1",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(AffectionLogRow {
                id: row.get(0)?,
                change_value: row.get(1)?,
                reason: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

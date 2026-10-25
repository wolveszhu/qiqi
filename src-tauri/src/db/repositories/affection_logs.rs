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
/// ⚠️ `created_at` 默认值为 `CURRENT_TIMESTAMP`（**UTC**），故过滤左值必须先
/// `,'localtime'` 转本地再与本地今日比较——否则 UTC+8 用户在本地 00:00–08:00
/// 两日期相差一天，当日流水漏判 → 打卡唯一性与抚摸日上限失效（F3-F4 回归缺陷，
/// 见 `tests/affection_it.rs::todays_uses_local_date_not_utc` 钉子）。
/// 同口径约定见 `messages::by_date`（注释已明示 UTC 存储 + localtime 过滤）。
///
/// 返回给前端的 `created_at` 经 `datetime(created_at,'localtime')` 转本地时区，
/// 避免前端把 UTC 当本地时间展示（UTC+8 慢 8 小时）。
pub fn todays(conn: &Connection) -> Result<Vec<AffectionLogRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, change_value, reason, datetime(created_at, 'localtime') AS created_at
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
///
/// `created_at` 经 `datetime(created_at,'localtime')` 转本地时区返回（理由同上）。
pub fn recent(conn: &Connection, limit: i64) -> Result<Vec<AffectionLogRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, change_value, reason, datetime(created_at, 'localtime') AS created_at
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn recent_returns_local_time_not_raw_utc() {
        let (_dir, conn) = fresh_db();
        // 存储为 UTC（与全应用约定一致）
        insert(&conn, 1, "check_in").expect("写流水");
        let raw: String = conn
            .query_row("SELECT created_at FROM affection_logs", [], |r| r.get(0))
            .expect("读裸值");
        let converted: String = conn
            .query_row("SELECT datetime(created_at, 'localtime') FROM affection_logs", [], |r| {
                r.get(0)
            })
            .expect("读转换值");
        let rows = recent(&conn, 10).expect("读明细");
        assert_eq!(rows[0].created_at, converted, "recent 应返回本地时间而非 UTC 原文");
        assert!(chrono::NaiveDateTime::parse_from_str(&raw, "%Y-%m-%d %H:%M:%S").is_ok());
    }
}

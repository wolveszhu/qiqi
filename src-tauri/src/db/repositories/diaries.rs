//! diaries 表访问（C4 只读；C7.2 生成/重生成 upsert）。

use rusqlite::{Connection, params};

use crate::db::models::DiaryRow;
use crate::error::AppError;

/// 列出日记（按日期倒序，limit 防止全表拖拽）。
pub fn list(conn: &Connection, limit: i64) -> Result<Vec<DiaryRow>, AppError> {
    let mut stmt = conn
        .prepare(
            // id DESC 为次级排序（同一天的重生成场景也能稳定倒序）
            "SELECT id, diary_date, content, mood, created_at
             FROM diaries ORDER BY diary_date DESC, id DESC LIMIT ?1",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(DiaryRow {
                id: row.get(0)?,
                diary_date: row.get(1)?,
                content: row.get(2)?,
                mood: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 指定日期是否已有日记（启动补漏判重）。
pub fn exists(conn: &Connection, diary_date: &str) -> Result<bool, AppError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM diaries WHERE diary_date = ?1",
            params![diary_date],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;
    Ok(count > 0)
}

/// upsert 日记（diary_date UNIQUE → 覆盖当日；裁决 #11-5 重生成语义）。
/// 返回该行 id。
pub fn upsert(
    conn: &Connection,
    diary_date: &str,
    content: &str,
    mood: Option<i64>,
) -> Result<i64, AppError> {
    conn.execute(
        "INSERT INTO diaries (diary_date, content, mood) VALUES (?1, ?2, ?3)
         ON CONFLICT(diary_date) DO UPDATE SET
             content = excluded.content,
             mood = excluded.mood,
             updated_at = CURRENT_TIMESTAMP",
        params![diary_date, content, mood],
    )
    .map_err(AppError::from)?;
    Ok(conn.last_insert_rowid())
}

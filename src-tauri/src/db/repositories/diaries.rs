//! diaries 表访问（只读；生成/重生成属 C7）。

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

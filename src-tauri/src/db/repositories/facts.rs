//! facts 表访问（只读；编辑/删除与向量同步属 C7）。

use rusqlite::{Connection, params};

use crate::db::models::FactRow;
use crate::error::AppError;

/// 列出事实（按创建时间倒序，limit 防止全表拖拽）。
pub fn list(conn: &Connection, limit: i64) -> Result<Vec<FactRow>, AppError> {
    let mut stmt = conn
        .prepare(
            // id DESC 为次级排序：同秒创建的多条（created_at 相同）也能稳定倒序
            "SELECT id, category, content, confidence, created_at
             FROM facts ORDER BY created_at DESC, id DESC LIMIT ?1",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(FactRow {
                id: row.get(0)?,
                category: row.get(1)?,
                content: row.get(2)?,
                confidence: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

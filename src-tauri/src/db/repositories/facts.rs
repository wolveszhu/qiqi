//! facts 表访问：C4 蒸馏插入（返回 id 供 vec_memory rowid 关联）。

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

/// 插入事实并返回 id（category：0=preference, 1=event, 2=profile）。
pub fn insert(
    conn: &Connection,
    category: i64,
    content: &str,
    confidence: f64,
) -> Result<i64, AppError> {
    conn.execute(
        "INSERT INTO facts (category, content, confidence) VALUES (?1, ?2, ?3)",
        params![category, content, confidence],
    )
    .map_err(AppError::from)?;
    Ok(conn.last_insert_rowid())
}

/// 更新事实内容（updated_at 刷新；返回受影响行数，0 = 不存在）。
pub fn update(conn: &Connection, id: i64, content: &str) -> Result<usize, AppError> {
    conn.execute(
        "UPDATE facts SET content = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![content, id],
    )
    .map_err(AppError::from)
}

/// 删除事实（返回受影响行数，0 = 不存在）。
pub fn delete(conn: &Connection, id: i64) -> Result<usize, AppError> {
    conn.execute("DELETE FROM facts WHERE id = ?1", params![id]).map_err(AppError::from)
}

/// 近因事实（Prompt 拼装用：按更新时间倒序取 N 条 content）。
pub fn recent_contents(conn: &Connection, limit: i64) -> Result<Vec<String>, AppError> {
    let mut stmt = conn
        .prepare("SELECT content FROM facts ORDER BY updated_at DESC, id DESC LIMIT ?1")
        .map_err(AppError::from)?;
    let rows =
        stmt.query_map(params![limit], |row| row.get::<_, String>(0)).map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

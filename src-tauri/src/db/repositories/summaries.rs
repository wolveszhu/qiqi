//! summaries 表访问（C4 蒸馏：会话摘要 upsert；session_id UNIQUE）。

use rusqlite::{Connection, params};

use crate::error::AppError;

/// upsert 会话摘要（session_id UNIQUE → INSERT OR REPLACE）。
pub fn upsert(conn: &Connection, session_id: &str, summary_text: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO summaries (session_id, summary_text) VALUES (?1, ?2)
         ON CONFLICT(session_id) DO UPDATE SET summary_text = excluded.summary_text,
         updated_at = CURRENT_TIMESTAMP",
        params![session_id, summary_text],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 读取会话摘要（无则 None）。
pub fn get(conn: &Connection, session_id: &str) -> Result<Option<String>, AppError> {
    let mut stmt = conn
        .prepare("SELECT summary_text FROM summaries WHERE session_id = ?1")
        .map_err(AppError::from)?;
    let mut rows = stmt.query(params![session_id]).map_err(AppError::from)?;
    match rows.next().map_err(AppError::from)? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

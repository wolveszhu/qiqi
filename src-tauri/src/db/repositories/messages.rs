//! messages 表访问（C4 对话链路：插入用户/助手消息、最近 N 条上下文）。

use rusqlite::{Connection, params};

use crate::error::AppError;

/// 一条对话消息（上下文用）。
#[derive(Debug, Clone)]
pub struct MessageRow {
    pub id: i64,
    pub role: i64, // 0=user, 1=assistant（勘误#2）
    pub content: String,
}

/// 插入消息并返回 id。
pub fn insert(
    conn: &Connection,
    session_id: &str,
    role: i64,
    content: &str,
) -> Result<i64, AppError> {
    conn.execute(
        "INSERT INTO messages (session_id, role, content) VALUES (?1, ?2, ?3)",
        params![session_id, role, content],
    )
    .map_err(AppError::from)?;
    Ok(conn.last_insert_rowid())
}

/// 读取某会话最近 `limit` 条消息（时间正序返回，供 Prompt 拼装）。
pub fn recent(
    conn: &Connection,
    session_id: &str,
    limit: i64,
) -> Result<Vec<MessageRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, role, content FROM (
                 SELECT id, role, content FROM messages
                 WHERE session_id = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2
             ) ORDER BY id ASC",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![session_id, limit], |row| {
            Ok(MessageRow { id: row.get(0)?, role: row.get(1)?, content: row.get(2)? })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

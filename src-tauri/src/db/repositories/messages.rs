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

/// 会话列表项（窗口 B「对话」Tab 左侧列表数据源）。
#[derive(Debug, Clone)]
pub struct SessionRow {
    pub session_id: String,
    /// 标题预览：首条 user 消息（无 user 消息则退化首条消息，再无则空串 → UI 显示"新对话"）。
    pub preview: String,
    /// 最后活跃时间（MAX(created_at)，UTC 字符串）。
    pub last_at: String,
    pub message_count: i64,
}

/// 会话列表：按最后活跃倒序，preview 取各会话首条 user 消息（SQL 全参数化）。
pub fn list_sessions(conn: &Connection, limit: i64) -> Result<Vec<SessionRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT session_id,
                    MAX(created_at) AS last_at,
                    MAX(id)         AS last_id,
                    COUNT(*)        AS message_count,
                    COALESCE(
                        (SELECT m2.content FROM messages m2
                          WHERE m2.session_id = m.session_id AND m2.role = 0
                          ORDER BY m2.id ASC LIMIT 1),
                        (SELECT m3.content FROM messages m3
                          WHERE m3.session_id = m.session_id
                          ORDER BY m3.id ASC LIMIT 1),
                        ''
                    ) AS preview
             FROM messages m
             GROUP BY session_id
             ORDER BY last_at DESC, last_id DESC
             LIMIT ?1",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(SessionRow {
                session_id: row.get(0)?,
                last_at: row.get(1)?,
                message_count: row.get(3)?,
                preview: row.get(4)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 读取会话消息（时间正序，UI 展示用）。
///
/// 语义 = `recent`（取最近 `limit` 条后按 id 升序），不重复造 SQL：
/// 超长会话只回最后 `limit` 条，避免历史会话拖垮渲染。
pub fn full_by_session(
    conn: &Connection,
    session_id: &str,
    limit: i64,
) -> Result<Vec<MessageRow>, AppError> {
    recent(conn, session_id, limit)
}

/// 删除整个会话：messages + summaries **同一事务**。
///
/// 摘要必须连带删：`summaries.session_id` 为 UNIQUE，残留会让下次同 id 会话的
/// Prompt 拼进旧摘要（串味），且删除会话语义上就该清干净。
pub fn delete_session(conn: &Connection, session_id: &str) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction().map_err(AppError::from)?;
    tx.execute("DELETE FROM messages WHERE session_id = ?1", params![session_id])
        .map_err(AppError::from)?;
    super::summaries::delete(&tx, session_id)?;
    tx.commit().map_err(AppError::from)?;
    Ok(())
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

/// 读取指定**本地日期**的全部消息（时间正序；C7.2 日记输入）。
///
/// `created_at` 为 UTC（CURRENT_TIMESTAMP），按 `date(created_at,'localtime')` 过滤。
pub fn by_date(conn: &Connection, date: &str, limit: usize) -> Result<Vec<MessageRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, role, content FROM messages
             WHERE date(created_at, 'localtime') = ?1
             ORDER BY created_at ASC, id ASC LIMIT ?2",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![date, limit as i64], |row| {
            Ok(MessageRow { id: row.get(0)?, role: row.get(1)?, content: row.get(2)? })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 指定本地日期的消息条数（补漏判空，轻量 COUNT）。
pub fn count_by_date(conn: &Connection, date: &str) -> Result<i64, AppError> {
    conn.query_row(
        "SELECT COUNT(*) FROM messages WHERE date(created_at, 'localtime') = ?1",
        params![date],
        |row| row.get(0),
    )
    .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::summaries;
    use crate::test_util::fresh_db;

    #[test]
    fn list_sessions_orders_by_last_activity_with_preview() {
        let (_dir, conn) = fresh_db();
        // s1：两轮；s2：后建一轮（同秒插入 → 用 MAX(id) 兜底排序）
        insert(&conn, "s1", 0, "你好呀").expect("插入");
        insert(&conn, "s1", 1, "我在").expect("插入");
        insert(&conn, "s2", 0, "今天天气不错").expect("插入");

        let list = list_sessions(&conn, 20).expect("列会话");
        assert_eq!(list.len(), 2, "两个会话");
        assert_eq!(list[0].session_id, "s2", "最近活跃在前");
        assert_eq!(list[0].preview, "今天天气不错", "preview=首条 user 消息");
        assert_eq!(list[0].message_count, 1);
        assert_eq!(list[1].session_id, "s1");
        assert_eq!(list[1].preview, "你好呀", "preview 取首条 user，非 assistant");
        assert_eq!(list[1].message_count, 2);
        assert!(!list[0].last_at.is_empty(), "最后活跃时间非空");
    }

    #[test]
    fn delete_session_removes_messages_and_summary_together() {
        let (_dir, conn) = fresh_db();
        insert(&conn, "gone", 0, "要被删掉").expect("插入");
        insert(&conn, "gone", 1, "回复也要删").expect("插入");
        insert(&conn, "keep", 0, "保留的会话").expect("插入");
        summaries::upsert(&conn, "gone", "旧摘要").expect("写摘要");
        summaries::upsert(&conn, "keep", "保留摘要").expect("写摘要");

        delete_session(&conn, "gone").expect("删会话");

        let list = list_sessions(&conn, 20).expect("列会话");
        assert_eq!(list.len(), 1, "只剩 keep");
        assert_eq!(list[0].session_id, "keep");
        assert!(
            summaries::get(&conn, "gone").expect("读摘要").is_none(),
            "摘要必须连带删除，否则同 id 会话会串味"
        );
        assert!(summaries::get(&conn, "keep").expect("读摘要").is_some(), "其他会话摘要不受影响");
    }

    #[test]
    fn full_by_session_returns_ascending_messages() {
        let (_dir, conn) = fresh_db();
        insert(&conn, "s", 0, "第一句").expect("插入");
        insert(&conn, "s", 1, "第二句").expect("插入");
        insert(&conn, "s", 0, "第三句").expect("插入");

        let all = full_by_session(&conn, "s", 200).expect("读会话");
        let contents: Vec<&str> = all.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(contents, vec!["第一句", "第二句", "第三句"], "时间正序");
        assert!(all.iter().all(|m| m.id > 0));
    }
}

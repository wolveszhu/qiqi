//! vec_memory 虚拟表访问（C4 蒸馏写入 + 混合检索 KNN）。
//!
//! 契约（附录 A §A.1#5）：`rowid` = 来源行 id（facts.id/messages.id），
//! `+source_text` 辅助列冗余存文本（检索免回表）。

use rusqlite::{Connection, params};

use crate::error::AppError;

/// f32 向量 → sqlite-vec 期望的 blob 字节（little-endian）。
pub fn embedding_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// 写入向量（rowid = 来源 facts.id；source_text = 事实内容）。
pub fn insert(
    conn: &Connection,
    rowid: i64,
    embedding: &[u8],
    source_text: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO vec_memory (rowid, embedding, source_text) VALUES (?1, ?2, ?3)",
        params![rowid, embedding, source_text],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// KNN 检索：返回 (rowid, source_text) 列表（按距离升序）。
pub fn knn(conn: &Connection, query: &[u8], k: i64) -> Result<Vec<(i64, String)>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT rowid, source_text FROM vec_memory
             WHERE embedding MATCH ?1 AND k = ?2
             ORDER BY distance",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![query, k], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 删除来源行的向量（级联辅助；C7 编辑/删除时同步调用）。
pub fn delete_by_rowid(conn: &Connection, rowid: i64) -> Result<(), AppError> {
    conn.execute("DELETE FROM vec_memory WHERE rowid = ?1", params![rowid])
        .map_err(AppError::from)?;
    Ok(())
}

/// 全量重建第一步：DROP 后按维度重建虚拟表（C7.4；DDL 与 M2 单点同构）。
///
/// ⚠️ 连接必须已注册 sqlite-vec 扩展（生产连接由 `init` 保证）；
/// `dim` 显式传入（生产 = `config::EMBEDDING_DIM`，测试可用小维度配 Mock）。
pub fn drop_and_recreate(conn: &Connection, dim: usize) -> Result<(), AppError> {
    conn.execute_batch(&format!(
        "DROP TABLE IF EXISTS vec_memory;
         CREATE VIRTUAL TABLE vec_memory USING vec0 (
             embedding float[{dim}],
             +source_text TEXT
         );"
    ))
    .map_err(AppError::from)?;
    Ok(())
}

//! memory 命令域（薄层）：B5 只落**只读查询**；写操作签名预留，实现属 C7。
//!
//! 写操作（编辑/删除/重生成 + `vec_memory` 同步重建）必须**同事务**保证向量与行一致，
//! 该事务逻辑与 embed 调用在 C7 落位；本计划仅声明签名并返回"未接入"。

use tauri::State;

use crate::db::models::{DiaryDto, FactDto};
use crate::db::repositories::{diaries, facts};
use crate::error::AppError;
use crate::state::AppState;

/// 单次查询上限（C7 引入分页后改为参数化）。
const LIST_LIMIT: i64 = 200;

/// 列出结构化事实。
#[tauri::command]
pub fn memory_list_facts(state: State<'_, AppState>) -> Result<Vec<FactDto>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    let rows = facts::list(&conn, LIST_LIMIT)?;
    Ok(rows
        .into_iter()
        .map(|r| FactDto {
            id: r.id,
            category: r.category,
            content: r.content,
            confidence: r.confidence,
            created_at: r.created_at,
        })
        .collect())
}

/// 列出日记。
#[tauri::command]
pub fn memory_list_diaries(state: State<'_, AppState>) -> Result<Vec<DiaryDto>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    let rows = diaries::list(&conn, LIST_LIMIT)?;
    Ok(rows
        .into_iter()
        .map(|r| DiaryDto { id: r.id, diary_date: r.diary_date, content: r.content, mood: r.mood })
        .collect())
}

// ---------- [C7 接入]：签名已定，实现随记忆管道落位 ----------

/// 编辑事实（含向量同步）。
#[tauri::command]
pub fn memory_update_fact(id: i64, content: String) -> Result<(), AppError> {
    let _ = (id, content);
    Err(AppError::Config("[C7 接入] 记忆编辑与向量同步尚未接入".into()))
}

/// 删除事实（含向量级联删除）。
#[tauri::command]
pub fn memory_delete_fact(id: i64) -> Result<(), AppError> {
    let _ = id;
    Err(AppError::Config("[C7 接入] 记忆删除与向量级联尚未接入".into()))
}

/// 重新生成指定日期的日记（upsert 覆盖）。
#[tauri::command]
pub fn memory_regenerate_diary(date: String) -> Result<(), AppError> {
    let _ = date;
    Err(AppError::Config("[C7 接入] 日记生成尚未接入".into()))
}

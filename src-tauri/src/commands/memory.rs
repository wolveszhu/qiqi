//! memory 命令域（薄层）：只读查询（B5）+ 写操作实装（C7.3，向量同步在 llm 域）。
//!
//! 事务一致性（冷眼三问①）：编辑 = embed（无锁段，失败不触碰库）→ 同事务
//! 行更新 + 向量重写；删除 = 同事务行删除 + 向量级联。

use tauri::{AppHandle, State};

use rusqlite::Connection;

use crate::db::models::{DiaryDto, FactDto};
use crate::db::repositories::{app_settings, custom_model_settings, diaries, facts};
use crate::error::AppError;
use crate::llm::client::{LlmClient as _, ReqwestLlmClient};
use crate::state::AppState;

/// 单次查询上限（分页需求出现后改为参数化）。
const LIST_LIMIT: i64 = 200;

/// 列出结构化事实。
#[tauri::command]
pub fn memory_list_facts(state: State<'_, AppState>) -> Result<Vec<FactDto>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    memory_list_facts_inner(&conn)
}

/// 纯逻辑（F1.2 可直测）：事实列表（limit 固定）。
fn memory_list_facts_inner(conn: &Connection) -> Result<Vec<FactDto>, AppError> {
    let rows = facts::list(conn, LIST_LIMIT)?;
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
    memory_list_diaries_inner(&conn)
}

/// 纯逻辑（F1.2 可直测）：日记列表（按日期倒序）。
fn memory_list_diaries_inner(conn: &Connection) -> Result<Vec<DiaryDto>, AppError> {
    let rows = diaries::list(conn, LIST_LIMIT)?;
    Ok(rows
        .into_iter()
        .map(|r| DiaryDto { id: r.id, diary_date: r.diary_date, content: r.content, mood: r.mood })
        .collect())
}

// ---------- [C7.3 实装]：写操作与向量同步 ----------

/// 编辑事实（含向量同步）：embed（无锁）→ 同事务行更新 + 向量重写。
#[tauri::command]
pub async fn memory_update_fact(app: AppHandle, id: i64, content: String) -> Result<(), AppError> {
    use tauri::Manager as _;
    let state = app.state::<AppState>();

    // ① 配置快照（锁段一）
    let (tiered, quota_embedded, custom) = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        let tiered = app_settings::get_tiered(&conn)?;
        let quota = app_settings::get_quota_mode(&conn)?;
        let custom = custom_model_settings::list_enabled_llm(&conn)?;
        (tiered, quota == crate::config::QUOTA_MODE_EMBEDDED, custom)
    };

    // ② embed（无锁段；失败不触碰库）
    let client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);
    let memory_target =
        crate::llm::router::memory_target_from_sources(&tiered, quota_embedded, &custom);
    let embedding = client
        .embed(&memory_target, vec![content.clone()])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Llm("embed 返回空结果".into()))?;

    // ③ 同事务写库（锁段二）
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    crate::llm::distill::apply_fact_embedding(&conn, id, &content, &embedding)
}

/// 删除事实（含向量级联删除；纯 DB 同事务，无 LLM 调用）。
#[tauri::command]
pub fn memory_delete_fact(app: AppHandle, id: i64) -> Result<(), AppError> {
    use tauri::Manager as _;
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    crate::llm::distill::remove_fact(&conn, id)
}

/// 新增事实（D3.1 正向入口：用户在设置窗主动补记）。
///
/// 链路（同 C7.3）：校验 → embed（无锁段，失败不触碰库）→ 同事务 insert facts + 写 vec_memory。
/// category 越界归 0；embed 失败返回 `Llm`；内容保留在前端供重试。
#[tauri::command]
pub async fn memory_add_fact(
    app: AppHandle,
    content: String,
    category: i64,
) -> Result<(), AppError> {
    use tauri::Manager as _;
    let content = crate::llm::distill::validate_fact_content(&content)?;
    let category = if (0..=2).contains(&category) { category } else { 0 };

    let state = app.state::<AppState>();
    let (tiered, quota_embedded, custom) = {
        let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
        let tiered = app_settings::get_tiered(&conn)?;
        let quota = app_settings::get_quota_mode(&conn)?;
        let custom = custom_model_settings::list_enabled_llm(&conn)?;
        (tiered, quota == crate::config::QUOTA_MODE_EMBEDDED, custom)
    };

    let client = ReqwestLlmClient::new(crate::config::EMBEDDING_DIM);
    let memory_target =
        crate::llm::router::memory_target_from_sources(&tiered, quota_embedded, &custom);
    let embedding = client
        .embed(&memory_target, vec![content.clone()])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Llm("embed 返回空结果".into()))?;

    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    crate::llm::distill::add_fact_with_embedding(&conn, category, &content, &embedding)?;
    Ok(())
}

/// 重新生成指定日期的日记（upsert 覆盖；memory 档短生成）。
#[tauri::command]
pub async fn memory_regenerate_diary(app: AppHandle, date: String) -> Result<(), AppError> {
    // 生成管道内部三段式（锁不跨 await）；当日无消息 → Err(Config) 供 D3 提示
    let generated = crate::llm::diary::generate_for_date(&app, &date).await?;
    if generated {
        Ok(())
    } else {
        Err(AppError::Config(format!("{date} 没有对话记录，无法生成日记")))
    }
}

// F1.2：命令 inner 直调用（`inner(&conn)`）——验证只读查询的 DB 交互。
// 豁免：`memory_add/update/delete/regenerate` 需 `AppHandle` + embed/LLM 网络调用，
// 向量一致性与 upsert 幂等已由 `tests/memory_it.rs`（C7 集成）覆盖（见覆盖矩阵豁免登记）。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::{diaries, facts};
    use crate::test_util::fresh_db;

    #[test]
    fn memory_list_facts_reflects_seed() {
        let (_dir, conn) = fresh_db();
        facts::insert(&conn, 0, "喜欢喝冰美式", 0.9).unwrap();
        facts::insert(&conn, 1, "昨天加班到很晚", 0.8).unwrap();
        let got = memory_list_facts_inner(&conn).expect("查询事实");
        assert_eq!(got.len(), 2);
        assert!(got.iter().any(|f| f.content == "喜欢喝冰美式"));
        assert!(got.iter().all(|f| f.id > 0), "应带出主键");
    }

    #[test]
    fn memory_list_diaries_reflects_seed() {
        let (_dir, conn) = fresh_db();
        diaries::upsert(&conn, "2026-10-21", "今天用户陪我聊天啦", Some(0)).unwrap();
        let got = memory_list_diaries_inner(&conn).expect("查询日记");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].diary_date, "2026-10-21");
        assert_eq!(got[0].mood, Some(0));
    }
}

//! 联网搜索配置命令（薄层：参数校验 + 转发仓储，禁止业务逻辑）。
//!
//! 红线（code-standards §5 / settings.rs 同口径）：明文 Key 绝不出现在返回 DTO，
//! 只回 `has_key` + 末 4 位指纹；明文 Key 的合法路径仅：前端输入 → IPC 参数 → Rust 落库。

use tauri::State;

use rusqlite::Connection;

use crate::db::models::{SaveSearchSettingsPayload, SearchSettingsDto};
use crate::db::repositories::search_settings as repo;
use crate::error::AppError;
use crate::state::AppState;

/// 读取联网搜索配置（Key 脱敏回传）。
#[tauri::command]
pub fn search_settings_get(state: State<'_, AppState>) -> Result<SearchSettingsDto, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    get_inner(&conn)
}

/// 保存联网搜索配置（api_key 缺省 = 不改动存量；空串 = 清空）。
#[tauri::command]
pub fn search_settings_save(
    payload: SaveSearchSettingsPayload,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    save_inner(&conn, payload)
}

/// inner（薄壳 → inner，便于 `#[test]` 直调，F1.2）：组装脱敏 DTO。
fn get_inner(conn: &Connection) -> Result<SearchSettingsDto, AppError> {
    let row = repo::get(conn)?;
    // 先取借用型字段（避免 String 字段被移动后借用到 row）
    let has_key = row.has_key();
    let key_fingerprint = key_fingerprint(&row.api_key);
    let enabled = row.enabled;
    let max_results = row.max_results;
    let provider = row.provider;
    let lang = row.lang;
    Ok(SearchSettingsDto {
        provider,
        has_key,
        key_fingerprint,
        enabled,
        max_results,
        lang,
    })
}

/// inner（薄壳 → inner）：provider 校验已下沉至仓储 `save`，此处仅转发。
fn save_inner(conn: &Connection, payload: SaveSearchSettingsPayload) -> Result<(), AppError> {
    repo::save(
        conn,
        &payload.provider,
        payload.api_key.as_deref(),
        payload.enabled,
        payload.max_results,
        &payload.lang,
    )
}

/// Key 末 4 位指纹（脱敏口径与 settings 档位一致：单一真源）。
fn key_fingerprint(api_key: &str) -> Option<String> {
    if api_key.len() >= 4 {
        Some(api_key[api_key.len() - 4..].to_string())
    } else {
        None
    }
}

/// 数据库锁错误统一映射（与既有命令层口径一致）。
fn db_lock_err<T: std::fmt::Display>(e: T) -> AppError {
    AppError::Db(format!("获取数据库连接失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn get_never_leaks_plain_key() {
        let (_dir, conn) = fresh_db();
        repo::save(&conn, "tavily", Some("sk-SECRET-4321"), true, 5, "zh").expect("保存");
        let dto = get_inner(&conn).expect("读取");
        assert!(dto.has_key);
        assert_eq!(dto.key_fingerprint.as_deref(), Some("4321"));
        let json = serde_json::to_string(&dto).expect("序列化");
        assert!(!json.contains("sk-SECRET-4321"), "泄露明文 Key: {json}");
        assert!(!json.contains("SECRET"), "连 Key 主体亦不得出现: {json}");
    }

    #[test]
    fn save_roundtrip_and_unchanged_key() {
        let (_dir, conn) = fresh_db();
        repo::save(&conn, "tavily", Some("sk-OLD-1111"), true, 5, "zh").expect("保存");
        // 改 enabled/provider 但不传 key → 存量 Key 保留
        save_inner(
            &conn,
            SaveSearchSettingsPayload {
                provider: "brave".into(),
                api_key: None,
                enabled: false,
                max_results: 7,
                lang: "en".into(),
            },
        )
        .expect("保存");
        let dto = get_inner(&conn).expect("读取");
        assert_eq!(dto.provider, "brave");
        assert!(!dto.enabled);
        assert!(dto.has_key, "None Key 不应清空");
        assert_eq!(dto.max_results, 7);
    }
}

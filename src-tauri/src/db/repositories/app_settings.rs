//! app_settings 表访问（type=1 三档 LLM 配置，type=2 勿扰配置）。

use rusqlite::{Connection, params};

use crate::db::models::AppSettingsRow;
use crate::error::AppError;

/// 读取指定 type 的行（不存在返回 None）。
pub fn get_row(conn: &Connection, setting_type: i64) -> Result<Option<AppSettingsRow>, AppError> {
    let mut stmt = conn
        .prepare("SELECT id, type, is_custom_quota_used, value FROM app_settings WHERE type = ?1")
        .map_err(AppError::from)?;
    let mut rows = stmt.query(params![setting_type]).map_err(AppError::from)?;
    match rows.next().map_err(AppError::from)? {
        Some(row) => Ok(Some(AppSettingsRow {
            id: row.get(0)?,
            setting_type: row.get(1)?,
            is_custom_quota_used: row.get(2)?,
            value: row.get(3)?,
        })),
        None => Ok(None),
    }
}

/// 写入/更新指定 type 的 value（存在则 UPDATE，不存在则 INSERT）。
pub fn upsert_value(conn: &Connection, setting_type: i64, value: &str) -> Result<(), AppError> {
    let changed = conn
        .execute("UPDATE app_settings SET value = ?1 WHERE type = ?2", params![value, setting_type])
        .map_err(AppError::from)?;
    if changed == 0 {
        conn.execute(
            "INSERT INTO app_settings (type, is_custom_quota_used, value) VALUES (?1, 1, ?2)",
            params![setting_type, value],
        )
        .map_err(AppError::from)?;
    }
    Ok(())
}

/// 读取额度模式（1=内置额度 / 2=BYOK；缺省 1）。
pub fn get_quota_mode(conn: &Connection) -> Result<i64, AppError> {
    Ok(get_row(conn, 1)?.map(|r| r.is_custom_quota_used).unwrap_or(1))
}

/// 设置额度模式（作用于 type=1 行；行不存在则创建空 value 行）。
pub fn set_quota_mode(conn: &Connection, mode: i64) -> Result<(), AppError> {
    let changed = conn
        .execute("UPDATE app_settings SET is_custom_quota_used = ?1 WHERE type = 1", params![mode])
        .map_err(AppError::from)?;
    if changed == 0 {
        conn.execute(
            "INSERT INTO app_settings (type, is_custom_quota_used, value) VALUES (1, ?1, NULL)",
            params![mode],
        )
        .map_err(AppError::from)?;
    }
    Ok(())
}

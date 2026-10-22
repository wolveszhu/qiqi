//! app_settings 表访问（type=1 三档 LLM 配置，type=2 勿扰配置）。

use rusqlite::{Connection, params};

use crate::config::{TIER_FALLBACK, TIER_MAIN, TIER_MEMORY};
use crate::db::models::{AppSettingsRow, LlmTierValue, TieredValue};
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

// ---------- C3 tiered 读写辅助 ----------

/// 读取三档配置（type=1 的 value）。
///
/// 双格式兼容（勘误 #7 兜底）：tiered 优先；旧扁平结构整体视为 main 档，
/// memory/fallback 为空（用户在 D2 补配；未配档位由 C4 router 回落）。
pub fn get_tiered(conn: &Connection) -> Result<TieredValue, AppError> {
    let raw = get_row(conn, 1)?.and_then(|r| r.value);
    parse_tiered(raw.as_deref())
}

/// 读取单档配置（tier 非法返回错误；未配置返回空值）。
pub fn get_tier(conn: &Connection, tier: &str) -> Result<LlmTierValue, AppError> {
    let tiered = get_tiered(conn)?;
    match tier {
        TIER_MAIN => Ok(tiered.main),
        TIER_MEMORY => Ok(tiered.memory),
        TIER_FALLBACK => Ok(tiered.fallback),
        other => Err(AppError::Config(format!("未知档位: {other}"))),
    }
}

/// 保存单档配置（读-改-写整包；tier 非法返回错误）。
///
/// 注：value 为整包 JSON，单档保存必然重写整包——并发场景由命令层单次锁保护。
pub fn save_tier(conn: &Connection, tier: &str, value: &LlmTierValue) -> Result<(), AppError> {
    let mut tiered = get_tiered(conn)?;
    match tier {
        TIER_MAIN => tiered.main = value.clone(),
        TIER_MEMORY => tiered.memory = value.clone(),
        TIER_FALLBACK => tiered.fallback = value.clone(),
        other => return Err(AppError::Config(format!("未知档位: {other}"))),
    }
    let json = serde_json::to_string(&tiered)?;
    upsert_value(conn, 1, &json)
}

/// BYOK 前置校验：main 档必须已配置 Key（切 BYOK 时的最低门槛）。
pub fn main_tier_has_key(conn: &Connection) -> Result<bool, AppError> {
    Ok(!get_tier(conn, TIER_MAIN)?.api_key.is_empty())
}

/// 解析 type=1 value：tiered 优先，回落扁平旧结构（勘误 #7 兜底）。
///
/// **legacy 兜底保留理由（F4 清理裁决）**：M3 迁移已使新库 type=1 为三档结构，运行时不该
/// 遇到扁平格式；保留仅为防御——从 M3 之前的备份恢复 `pet.db` 时仍可读而非硬失败。
pub(crate) fn parse_tiered(raw: Option<&str>) -> Result<TieredValue, AppError> {
    use crate::db::models::LegacyFlatValue;
    let Some(raw) = raw else {
        return Ok(TieredValue::default());
    };
    // 先判结构再解析（TieredValue 字段带 default，直接解析会把旧格式误判为三档全空）
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if value.get(TIER_MAIN).is_some()
        || value.get(TIER_MEMORY).is_some()
        || value.get(TIER_FALLBACK).is_some()
    {
        return Ok(serde_json::from_value::<TieredValue>(value)?);
    }
    let legacy: LegacyFlatValue = serde_json::from_value(value)
        .map_err(|e| AppError::Config(format!("app_settings(type=1) 格式无法识别: {e}")))?;
    Ok(TieredValue {
        main: crate::db::models::LlmTierValue {
            base_url: legacy.llm_base_url,
            model: legacy.llm_model,
            api_key: legacy.llm_api_key,
            model_custom: legacy.llm_model_custom,
        },
        ..Default::default()
    })
}

//! app_settings 表访问（type=1 三档 LLM 配置，type=2 勿扰配置，type=3 桌宠交互配置）。

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

// ---------- type=3 桌宠交互配置（边缘吸附休眠开关） ----------

/// type=3 行：桌宠交互配置（当前仅「边缘吸附休眠」开关；后续交互开关可并行追加键）。
pub const SETTING_TYPE_PET_INTERACTION: i64 = 3;
/// type=3 value 的键名（camelCase，与快照 DTO 对齐）。
pub const KEY_SNAP_SLEEP_ENABLED: &str = "snapSleepEnabled";

/// 读取边缘吸附休眠开关（**缺省/坏值均回落 true**：保持"拖到边缘即休眠"的现有行为）。
///
/// 解析失败/行不存在/键缺失 → true（宁可误触发也不静默失效）。
pub fn get_snap_sleep_enabled(conn: &Connection) -> Result<bool, AppError> {
    let raw = get_row(conn, SETTING_TYPE_PET_INTERACTION)?.and_then(|r| r.value);
    let Some(raw) = raw else {
        return Ok(true);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Ok(true); // 坏 JSON：回落默认
    };
    Ok(value.get(KEY_SNAP_SLEEP_ENABLED).and_then(|v| v.as_bool()).unwrap_or(true))
}

/// 写入边缘吸附休眠开关（整包覆盖 type=3 行；幂等）。
pub fn set_snap_sleep_enabled(conn: &Connection, enabled: bool) -> Result<(), AppError> {
    let value = serde_json::json!({ KEY_SNAP_SLEEP_ENABLED: enabled }).to_string();
    upsert_value(conn, SETTING_TYPE_PET_INTERACTION, &value)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn snap_sleep_defaults_to_true_when_row_missing() {
        let (_dir, conn) = fresh_db();
        assert!(
            get_snap_sleep_enabled(&conn).expect("读取"),
            "无 type=3 行 → 默认开启（保持现有行为）"
        );
    }

    #[test]
    fn snap_sleep_roundtrip_and_idempotent() {
        let (_dir, conn) = fresh_db();
        set_snap_sleep_enabled(&conn, false).expect("写 false");
        assert!(!get_snap_sleep_enabled(&conn).expect("读取"), "应读到 false");

        set_snap_sleep_enabled(&conn, false).expect("重复写 false（幂等）");
        assert!(!get_snap_sleep_enabled(&conn).expect("读取"));

        set_snap_sleep_enabled(&conn, true).expect("写 true");
        assert!(get_snap_sleep_enabled(&conn).expect("读取"), "应读到 true");

        // 只应存在一行 type=3（upsert 语义，不累积）
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM app_settings WHERE type = ?1",
                params![SETTING_TYPE_PET_INTERACTION],
                |row| row.get(0),
            )
            .expect("计数");
        assert_eq!(count, 1, "type=3 应只有一行");
    }

    #[test]
    fn snap_sleep_tolerates_garbage_value() {
        let (_dir, conn) = fresh_db();
        upsert_value(&conn, SETTING_TYPE_PET_INTERACTION, "not-json").expect("写坏值");
        assert!(
            get_snap_sleep_enabled(&conn).expect("读取"),
            "坏 JSON → 回落默认开启（不静默失效）"
        );
    }
}

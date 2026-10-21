//! settings 命令域（薄层：参数校验 + 转发仓储，禁止业务逻辑）。
//!
//! 红线（code-standards §5 / 设计方案）：
//! - **明文 Key 绝不出现在返回 DTO、日志、错误信息中**——只回 `has_key` + 末 4 位指纹；
//! - 明文 Key 的合法路径仅：前端输入框内存 → IPC 参数 → Rust 落库（库已全库加密）。

use tauri::State;

use crate::db::models::{
    DndValue, LegacyDndValue, LegacyFlatValue, LlmTierSettingsDto, LlmTierValue, SaveTierPayload,
    SettingsDndValue, SettingsSnapshotDto, TieredValue,
};
use crate::db::repositories::app_settings;
use crate::error::AppError;
use crate::state::AppState;

/// LLM 配置档位（与 `TieredValue` 三档一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierName {
    Main,
    Memory,
    Fallback,
}

impl TierName {
    /// 解析前端传入的档位名；非法值返回错误（薄层参数校验职责）。
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        match raw {
            "main" => Ok(Self::Main),
            "memory" => Ok(Self::Memory),
            "fallback" => Ok(Self::Fallback),
            other => Err(AppError::Config(format!("未知档位: {other}"))),
        }
    }
}

/// 读取设置快照（Key 脱敏回传）。
#[tauri::command]
pub fn settings_get_typed(state: State<'_, AppState>) -> Result<SettingsSnapshotDto, AppError> {
    // 锁的生命周期：单次获取、用完即释（禁嵌套，防死锁）
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;

    let llm_row = app_settings::get_row(&conn, 1)?;
    let dnd_row = app_settings::get_row(&conn, 2)?;
    let quota_mode = app_settings::get_quota_mode(&conn)?;

    let tiered = parse_tiered(llm_row.as_ref().and_then(|r| r.value.as_deref()))?;
    let dnd = parse_dnd(dnd_row.as_ref().and_then(|r| r.value.as_deref()))?;

    Ok(SettingsSnapshotDto {
        main: to_dto(&tiered.main),
        memory: to_dto(&tiered.memory),
        fallback: to_dto(&tiered.fallback),
        is_custom_quota_used: quota_mode,
        dnd,
    })
}

/// 保存单档配置（api_key 为空/缺省 = 不改动存量 Key）。
#[tauri::command]
pub fn settings_save_typed(
    tier: String,
    payload: SaveTierPayload,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let tier_name = TierName::parse(&tier)?;
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;

    let existing = app_settings::get_row(&conn, 1)?;
    let mut tiered = parse_tiered(existing.as_ref().and_then(|r| r.value.as_deref()))?;

    let target = match tier_name {
        TierName::Main => &mut tiered.main,
        TierName::Memory => &mut tiered.memory,
        TierName::Fallback => &mut tiered.fallback,
    };
    target.base_url = payload.base_url;
    target.model = payload.model;
    target.model_custom = payload.model_custom;
    // Key 仅在用户实际输入新值时覆盖；空串视为"不改动"
    match payload.api_key {
        Some(key) if !key.is_empty() => target.api_key = key,
        _ => {},
    }

    let json = serde_json::to_string(&tiered)?;
    app_settings::upsert_value(&conn, 1, &json)
}

/// 读取 type=1 的 value：优先三档结构，回落旧扁平格式（M1 种子兼容）。
fn parse_tiered(raw: Option<&str>) -> Result<TieredValue, AppError> {
    let Some(raw) = raw else {
        return Ok(TieredValue::default());
    };
    // 先判定结构再解析：TieredValue 各字段带 `#[serde(default)]`，
    // 若直接尝试解析，旧扁平 JSON 会被误判为"三档全空"（单测实测坑）。
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if value.get("main").is_some()
        || value.get("memory").is_some()
        || value.get("fallback").is_some()
    {
        return Ok(serde_json::from_value::<TieredValue>(value)?);
    }
    // 旧格式（扁平单档）：映射为 main 档，其余为空
    let legacy: LegacyFlatValue = serde_json::from_value(value)
        .map_err(|e| AppError::Config(format!("app_settings(type=1) 格式无法识别: {e}")))?;
    Ok(TieredValue {
        main: LlmTierValue {
            base_url: legacy.llm_base_url,
            model: legacy.llm_model,
            api_key: legacy.llm_api_key,
            model_custom: legacy.llm_model_custom,
        },
        ..Default::default()
    })
}

/// 读取勿扰配置：优先 `{"dnd":{...}}`，回落旧单段格式。
fn parse_dnd(raw: Option<&str>) -> Result<DndValue, AppError> {
    let Some(raw) = raw else {
        return Ok(DndValue::default());
    };
    // 同 parse_tiered：先判结构，避免带 default 的结构体误吞旧格式
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if value.get("dnd").is_some() {
        return Ok(serde_json::from_value::<SettingsDndValue>(value)?.dnd);
    }
    let legacy: LegacyDndValue = serde_json::from_value(value)
        .map_err(|e| AppError::Config(format!("app_settings(type=2) 格式无法识别: {e}")))?;
    Ok(DndValue {
        enabled: legacy.dnd_enabled,
        ranges: if legacy.dnd_start_time.is_empty() || legacy.dnd_end_time.is_empty() {
            Vec::new()
        } else {
            vec![crate::db::models::DndRange {
                start: legacy.dnd_start_time,
                end: legacy.dnd_end_time,
            }]
        },
    })
}

/// 库内档位 → 对外 DTO（脱敏：仅 has_key + 末 4 位指纹）。
///
/// 指纹口径按设计方案取**末 4 位**；如需更强脱敏，只改本函数（备选 SHA-256 前 8 位）。
fn to_dto(tier: &LlmTierValue) -> LlmTierSettingsDto {
    let has_key = !tier.api_key.is_empty();
    let key_fingerprint = if has_key && tier.api_key.len() >= 4 {
        Some(tier.api_key[tier.api_key.len() - 4..].to_string())
    } else {
        None
    };
    LlmTierSettingsDto {
        base_url: tier.base_url.clone(),
        model: tier.model.clone(),
        has_key,
        key_fingerprint,
        model_custom: tier.model_custom.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_name_parses_known_values() {
        assert_eq!(TierName::parse("main").expect("解析"), TierName::Main);
        assert_eq!(TierName::parse("memory").expect("解析"), TierName::Memory);
        assert_eq!(TierName::parse("fallback").expect("解析"), TierName::Fallback);
        assert!(TierName::parse("other").is_err(), "非法档位应报错");
    }

    #[test]
    fn to_dto_never_leaks_full_key() {
        let tier = LlmTierValue {
            base_url: "https://x/v1".into(),
            model: "m".into(),
            api_key: "sk-abcdef1234".into(),
            model_custom: String::new(),
        };
        let dto = to_dto(&tier);
        assert!(dto.has_key);
        assert_eq!(dto.key_fingerprint.as_deref(), Some("1234"), "仅末 4 位");
        // DTO 序列化后不得包含明文 Key
        let json = serde_json::to_string(&dto).expect("序列化");
        assert!(!json.contains("sk-abcdef1234"), "回包泄露明文 Key: {json}");
    }

    #[test]
    fn empty_key_means_no_key_and_no_fingerprint() {
        let dto = to_dto(&LlmTierValue::default());
        assert!(!dto.has_key);
        assert!(dto.key_fingerprint.is_none());
    }

    #[test]
    fn parse_tiered_falls_back_to_legacy_flat() {
        let legacy = r#"{"llm_base_url":"https://api.openai.com/v1","llm_api_key":"sk-x","llm_model":"gpt-4o-mini","llm_model_custom":""}"#;
        let tiered = parse_tiered(Some(legacy)).expect("兼容解析");
        assert_eq!(tiered.main.base_url, "https://api.openai.com/v1");
        assert_eq!(tiered.main.api_key, "sk-x");
        assert!(tiered.memory.model.is_empty(), "旧格式其余档为空");
    }

    #[test]
    fn parse_dnd_falls_back_to_legacy_single_range() {
        let legacy = r#"{"dnd_enabled":true,"dnd_start_time":"09:00","dnd_end_time":"18:00"}"#;
        let dnd = parse_dnd(Some(legacy)).expect("兼容解析");
        assert!(dnd.enabled);
        assert_eq!(dnd.ranges.len(), 1);
        assert_eq!(dnd.ranges[0].start, "09:00");
    }
}

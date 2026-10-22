//! settings 命令域（薄层：参数校验 + 转发仓储，禁止业务逻辑）。
//!
//! 红线（code-standards §5 / 设计方案）：
//! - **明文 Key 绝不出现在返回 DTO、日志、错误信息中**——只回 `has_key` + 末 4 位指纹；
//! - 明文 Key 的合法路径仅：前端输入框内存 → IPC 参数 → Rust 落库（库已全库加密）。

use tauri::State;

use rusqlite::Connection;

use crate::config::{
    PROVIDER_PRESETS, QUOTA_MODE_BYOK, QUOTA_MODE_EMBEDDED, TIER_FALLBACK, TIER_MAIN, TIER_MEMORY,
};
use crate::db::models::{
    DndValue, LegacyDndValue, LlmTierSettingsDto, LlmTierValue, ProviderPresetDto, SaveTierPayload,
    SettingsDndValue, SettingsSnapshotDto,
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
            TIER_MAIN => Ok(Self::Main),
            TIER_MEMORY => Ok(Self::Memory),
            TIER_FALLBACK => Ok(Self::Fallback),
            other => Err(AppError::Config(format!("未知档位: {other}"))),
        }
    }
}

/// 服务商预设列表（C3：经 IPC 下发，**前端不复制列表**——单一真源防漂移）。
///
/// 回包**不含任何 Key 信息**（preset 本就是公开配置）。
#[tauri::command]
pub fn settings_list_providers() -> Vec<ProviderPresetDto> {
    PROVIDER_PRESETS
        .iter()
        .map(|p| ProviderPresetDto {
            id: p.id.into(),
            label: p.label.into(),
            base_url: p.base_url.into(),
            default_model: p.default_model.into(),
            required: p.required,
            cost_tier: p.cost_tier.into(),
        })
        .collect()
}

/// 切换额度模式（1=内置额度 / 2=BYOK）。
///
/// BYOK 校验：main 档必须已配置 Key，否则 `AppError::Config`——
/// 无主对话 Key 时切 BYOK 会导致对话全链路不可用（父计划选型方案约束）。
#[tauri::command]
pub fn settings_set_quota_mode(mode: i64, state: State<'_, AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    set_quota_mode_inner(&conn, mode)
}

/// 纯逻辑（F1.2 可直测）：参数校验 + BYOK 前置检查 + 落库。
fn set_quota_mode_inner(conn: &Connection, mode: i64) -> Result<(), AppError> {
    if mode != QUOTA_MODE_EMBEDDED && mode != QUOTA_MODE_BYOK {
        return Err(AppError::Config(format!("未知额度模式: {mode}")));
    }
    if mode == QUOTA_MODE_BYOK && !app_settings::main_tier_has_key(conn)? {
        return Err(AppError::Config("切换自带 Key 前请先配置主对话档（main）的 API Key".into()));
    }
    app_settings::set_quota_mode(conn, mode)
}

/// 读取设置快照（Key 脱敏回传）。
#[tauri::command]
pub fn settings_get_typed(state: State<'_, AppState>) -> Result<SettingsSnapshotDto, AppError> {
    // 锁的生命周期：单次获取、用完即释（禁嵌套，防死锁）
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    settings_get_inner(&conn)
}

/// 纯逻辑（F1.2 可直测）：组装脱敏快照。
fn settings_get_inner(conn: &Connection) -> Result<SettingsSnapshotDto, AppError> {
    let dnd_row = app_settings::get_row(conn, 2)?;
    let quota_mode = app_settings::get_quota_mode(conn)?;
    let tiered = app_settings::get_tiered(conn)?;
    let dnd_full = parse_dnd_full(dnd_row.as_ref().and_then(|r| r.value.as_deref()))?;

    Ok(SettingsSnapshotDto {
        main: to_dto(&tiered.main),
        memory: to_dto(&tiered.memory),
        fallback: to_dto(&tiered.fallback),
        is_custom_quota_used: quota_mode,
        dnd: dnd_full.dnd,
        frequency: dnd_full.frequency,
    })
}

/// 保存单档配置（api_key 为空/缺省 = 不改动存量 Key）。
#[tauri::command]
pub fn settings_save_typed(
    tier: String,
    payload: SaveTierPayload,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    save_tier_inner(&conn, &tier, payload)
}

/// 纯逻辑（F1.2 可直测）：档位校验 + 读-改-写（Key 空串视为不改动）。
fn save_tier_inner(
    conn: &Connection,
    tier: &str,
    payload: SaveTierPayload,
) -> Result<(), AppError> {
    // 档位合法性校验（与仓储白名单同一常量；错误口径一致）
    TierName::parse(tier)?;

    // 读-改-写经仓储单点（tiered 解析/双格式兼容只存在于仓储层）
    let mut current = app_settings::get_tier(conn, tier)?;
    current.base_url = payload.base_url;
    current.model = payload.model;
    current.model_custom = payload.model_custom;
    // Key 仅在用户实际输入新值时覆盖；空串视为"不改动"
    match payload.api_key {
        Some(key) if !key.is_empty() => current.api_key = key,
        _ => {},
    }
    app_settings::save_tier(conn, tier, &current)
}

/// 保存勿扰 + 频率档位（type=2 行；frequency 同段持久化，避免跨行并发）。
#[tauri::command]
pub fn settings_save_dnd(
    dnd: DndValue,
    frequency: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    save_dnd_inner(&conn, dnd, frequency)
}

/// 纯逻辑（F1.2 可直测）：频率档位白名单校验 + type=2 行 upsert。
fn save_dnd_inner(conn: &Connection, dnd: DndValue, frequency: String) -> Result<(), AppError> {
    if !["quiet", "normal", "sticky"].contains(&frequency.as_str()) {
        return Err(AppError::Config(format!("未知频率档位: {frequency}")));
    }
    let payload = SettingsDndValue { dnd, frequency };
    let json = serde_json::to_string(&payload)?;
    app_settings::upsert_value(conn, 2, &json)
}

/// 读取勿扰配置（完整 type=2 行）：优先 `{"dnd":{...},"frequency":...}`，回落旧单段格式。
/// （type=1 的 tiered 解析已下沉至仓储层 `app_settings::get_tiered`，勿留双实现。）
fn parse_dnd_full(raw: Option<&str>) -> Result<SettingsDndValue, AppError> {
    let Some(raw) = raw else {
        return Ok(SettingsDndValue::default());
    };
    // 同 parse_tiered：先判结构，避免带 default 的结构体误吞旧格式
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if value.get("dnd").is_some() {
        return Ok(serde_json::from_value::<SettingsDndValue>(value)?);
    }
    let legacy: LegacyDndValue = serde_json::from_value(value)
        .map_err(|e| AppError::Config(format!("app_settings(type=2) 格式无法识别: {e}")))?;
    Ok(SettingsDndValue {
        dnd: DndValue {
            enabled: legacy.dnd_enabled,
            ranges: if legacy.dnd_start_time.is_empty() || legacy.dnd_end_time.is_empty() {
                Vec::new()
            } else {
                vec![crate::db::models::DndRange {
                    start: legacy.dnd_start_time,
                    end: legacy.dnd_end_time,
                }]
            },
        },
        frequency: String::new(),
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
        assert_eq!(TierName::parse(TIER_MAIN).expect("解析"), TierName::Main);
        assert_eq!(TierName::parse(TIER_MEMORY).expect("解析"), TierName::Memory);
        assert_eq!(TierName::parse(TIER_FALLBACK).expect("解析"), TierName::Fallback);
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
    fn parse_dnd_falls_back_to_legacy_single_range() {
        let legacy = r#"{"dnd_enabled":true,"dnd_start_time":"09:00","dnd_end_time":"18:00"}"#;
        let dnd = parse_dnd_full(Some(legacy)).expect("兼容解析").dnd;
        assert!(dnd.enabled);
        assert_eq!(dnd.ranges.len(), 1);
        assert_eq!(dnd.ranges[0].start, "09:00");
    }

    #[test]
    fn providers_dto_carries_no_key_material() {
        // C3 红线：preset 经 IPC 下发但不含任何 Key 信息
        let json = serde_json::to_string(&settings_list_providers()).expect("序列化");
        assert!(!json.contains("apiKey"), "preset 不应含 Key 字段: {json}");
        assert!(!json.contains("api_key"), "preset 不应含 Key 字段: {json}");
    }

    #[test]
    fn settings_command_roundtrip_never_leaks_key() {
        // F1.2：Key 脱敏红线——inner save→get，序列化回包绝不含明文 Key
        let (_dir, conn) = crate::test_util::fresh_db();
        save_tier_inner(
            &conn,
            TIER_MAIN,
            SaveTierPayload {
                base_url: "https://api.example/v1".into(),
                model: "gpt-test".into(),
                api_key: Some("sk-SECRET-1234".into()),
                model_custom: String::new(),
            },
        )
        .expect("保存 main 档");

        let snap = settings_get_inner(&conn).expect("读取快照");
        assert!(snap.main.has_key, "应识别已配置 Key");
        assert_eq!(snap.main.key_fingerprint.as_deref(), Some("1234"));
        let json = serde_json::to_string(&snap).expect("序列化");
        assert!(!json.contains("sk-SECRET-1234"), "快照泄露明文 Key: {json}");
        assert!(!json.contains("SECRET"), "连 Key 主体亦不得出现: {json}");
    }

    #[test]
    fn save_tier_empty_key_preserves_existing() {
        // 读-改-写红线：api_key 空串视为"不改动"，存量 Key 不被清空
        let (_dir, conn) = crate::test_util::fresh_db();
        save_tier_inner(
            &conn,
            TIER_MAIN,
            SaveTierPayload {
                base_url: "https://a/v1".into(),
                model: "m".into(),
                api_key: Some("sk-KEEP-9999".into()),
                model_custom: String::new(),
            },
        )
        .expect("首次保存");
        save_tier_inner(
            &conn,
            TIER_MAIN,
            SaveTierPayload {
                base_url: "https://a/v2".into(),
                model: "m2".into(),
                api_key: None, // 未提供 → 保留
                model_custom: String::new(),
            },
        )
        .expect("二次保存");
        let snap = settings_get_inner(&conn).expect("读取");
        assert_eq!(snap.main.base_url, "https://a/v2", "base_url 应更新");
        assert!(snap.main.has_key, "空串 Key 不应清空存量");
        assert_eq!(snap.main.key_fingerprint.as_deref(), Some("9999"));
    }

    #[test]
    fn save_tier_rejects_unknown_tier() {
        let (_dir, conn) = crate::test_util::fresh_db();
        let err = save_tier_inner(
            &conn,
            "bogus",
            SaveTierPayload {
                base_url: String::new(),
                model: String::new(),
                api_key: None,
                model_custom: String::new(),
            },
        );
        assert!(err.is_err(), "未知档位应报错");
    }

    #[test]
    fn settings_set_quota_mode_guards_byok_without_key() {
        let (_dir, conn) = crate::test_util::fresh_db();
        // 未知模式 → 拒绝
        assert!(set_quota_mode_inner(&conn, 99).is_err(), "未知额度模式应报错");
        // 未配置 main Key → 切 BYOK 被拒（避免对话链路不可用）
        assert!(set_quota_mode_inner(&conn, QUOTA_MODE_BYOK).is_err(), "无 main Key 不得切 BYOK");
        // 配置 Key 后 → BYOK 可切，快照回读一致
        save_tier_inner(
            &conn,
            TIER_MAIN,
            SaveTierPayload {
                base_url: "https://api.example/v1".into(),
                model: "m".into(),
                api_key: Some("sk-1234".into()),
                model_custom: String::new(),
            },
        )
        .expect("保存 main 档");
        set_quota_mode_inner(&conn, QUOTA_MODE_BYOK).expect("切 BYOK");
        let snap = settings_get_inner(&conn).expect("读取快照");
        assert_eq!(snap.is_custom_quota_used, QUOTA_MODE_BYOK);
    }

    #[test]
    fn settings_save_dnd_rejects_unknown_frequency() {
        let (_dir, conn) = crate::test_util::fresh_db();
        assert!(
            save_dnd_inner(&conn, DndValue::default(), "ultra".into()).is_err(),
            "未知频率档位应报错"
        );
        save_dnd_inner(&conn, DndValue::default(), "quiet".into()).expect("合法频率档位应保存");
    }
}

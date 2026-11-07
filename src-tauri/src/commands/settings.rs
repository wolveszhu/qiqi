//! settings 命令域（薄层：参数校验 + 转发仓储，禁止业务逻辑）。
//!
//! 红线（code-standards §5 / 设计方案）：
//! - **明文 Key 绝不出现在返回 DTO、日志、错误信息中**——只回 `has_key` + 末 4 位指纹；
//! - 明文 Key 的合法路径仅：前端输入框内存 → IPC 参数 → Rust 落库（库已全库加密）。

use std::collections::HashMap;

use tauri::{AppHandle, State};

use rusqlite::Connection;

use crate::config::{
    PROVIDER_PRESETS, QUOTA_MODE_BYOK, QUOTA_MODE_EMBEDDED, TIER_FALLBACK, TIER_MAIN, TIER_MEMORY,
};
use crate::db::models::{
    CustomModelSettingsDto, DndValue, LegacyDndValue, LlmTierSettingsDto, LlmTierValue,
    ProviderPresetDto, SaveCustomModelPayload, SaveTierPayload, SettingsDndValue,
    SettingsSnapshotDto,
};
use crate::db::repositories::{
    app_settings::{self, SNAP_SHOW_RATIO_DEFAULT, SNAP_SHOW_RATIO_MAX, SNAP_SHOW_RATIO_MIN},
    custom_model_settings,
};
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
            default_models: p.default_models.iter().map(|s| (*s).to_string()).collect(),
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
    // BYOK 前置：必须有可用 Key 来源，否则该模式下编排链无目标、对话直接落到本地兜底话术。
    // 三档（main）已被自定义平台取代、UI 不再暴露，故以**自定义平台**为准；
    // legacy main 档仍有 Key 的存量库一并放行（兼容）。
    if mode == QUOTA_MODE_BYOK
        && !(custom_model_settings::has_enabled_key(conn)?
            || app_settings::main_tier_has_key(conn)?)
    {
        return Err(AppError::Config(
            "切换自带 Key 前，请先在上方添加并启用一个自定义平台（填好 API Key）".into(),
        ));
    }
    app_settings::set_quota_mode(conn, mode)
}

/// 读取广告关闭映射（缺省空对象；前端按本地今日比较决定是否展示）。
#[tauri::command]
pub fn settings_get_ad_dismissals(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    app_settings::get_ad_dismissals(&conn)
}

/// 记录广告「今日不再显示」（adId 为素材唯一 ID；空值拒绝，避免写入无意义键）。
#[tauri::command]
pub fn settings_mark_ad_dismissed(
    ad_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    if ad_id.trim().is_empty() {
        return Err(AppError::Config("广告素材 ID 不能为空".into()));
    }
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    app_settings::mark_ad_dismissed(&conn, &ad_id)
}

/// 读取用户语言（缺省/坏值回落 zh-CN）。
#[tauri::command]
pub fn get_locale(state: State<'_, AppState>) -> Result<String, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    app_settings::get_locale(&conn)
}

/// 写入用户语言并即时重建托盘菜单（持 `AppHandle`，命令内直接重建比前端 emit 事件更可靠）。
///
/// 语言白名单在仓储层校验，非法值返回 `AppError::Config`，防注入/乱码。
#[tauri::command]
pub fn set_locale(
    locale: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    app_settings::set_locale(&conn, &locale)?;
    // 语言切换即时反映到系统托盘（无 WebView 渲染，纯 OS 菜单）。
    // 托盘重建失败非致命：locale 已落库，下次启动仍生效，故吞掉错误。
    let _ = crate::window::tray::rebuild_tray_menu(&app, &locale);
    Ok(())
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
        snap_sleep_enabled: app_settings::get_snap_sleep_enabled(conn)?,
        snap_show_ratio: app_settings::get_snap_show_ratio(conn)?,
    })
}

/// 边缘吸附休眠开关（type=3 行；前端 AvatarPanel 调用）。
#[tauri::command]
pub fn settings_set_snap_sleep(enabled: bool, state: State<'_, AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    set_snap_sleep_inner(&conn, enabled)
}

/// inner（薄壳 → inner，便于 `#[test]` 直调）。
fn set_snap_sleep_inner(conn: &Connection, enabled: bool) -> Result<(), AppError> {
    app_settings::set_snap_sleep_enabled(conn, enabled)
}

/// 吸附态露出比例（type=3 行；后端 clamp 到 [MIN, MAX] 后落库，返回落库值）。
#[tauri::command]
pub fn settings_set_snap_ratio(ratio: f64, state: State<'_, AppState>) -> Result<f64, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    set_snap_ratio_inner(&conn, ratio)
}

/// inner（薄壳 → inner，便于 `#[test]` 直调）：clamp 后落库并返回最终值。
fn set_snap_ratio_inner(conn: &Connection, ratio: f64) -> Result<f64, AppError> {
    let ratio = if ratio.is_finite() { ratio } else { SNAP_SHOW_RATIO_DEFAULT };
    let ratio = ratio.clamp(SNAP_SHOW_RATIO_MIN, SNAP_SHOW_RATIO_MAX);
    app_settings::set_snap_show_ratio(conn, ratio)?;
    Ok(ratio)
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
///
/// **legacy 兼容保留理由（F4 勘误 #7/#13 清理裁决）**：M4 迁移已使新库 type=2 为规范结构，
/// 运行时不该遇到旧格式；保留读取兼容仅为**防御**——用户从 M4 之前的备份恢复 `pet.db` 时
/// （备份未随迁移链前进），仍可正常读取而非硬失败。迁移负责"新库干净"，兼容负责"旧备份可读"。
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

// ---------- 自定义多模型管理（custom_model_settings；优先于内置） ----------

/// 自定义模型列表（按 sort_order；Key 脱敏回传）。
#[tauri::command]
pub fn settings_list_custom_models(
    state: State<'_, AppState>,
) -> Result<Vec<CustomModelSettingsDto>, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    list_custom_models_inner(&conn)
}

/// 保存自定义模型（id 缺省 = 新建并追加到队尾；返回保存后的 id）。
///
/// Key 语义：`api_key` 缺省（None）= 不改动存量；空串（Some("")）= 显式清空。
#[tauri::command]
pub fn settings_save_custom_model(
    payload: SaveCustomModelPayload,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    save_custom_model_inner(
        &conn,
        payload.id,
        &payload.name,
        &payload.base_url,
        &payload.models,
        payload.api_key.as_deref(),
        payload.enabled.unwrap_or(true),
    )
}

/// 删除自定义模型。
#[tauri::command]
pub fn settings_delete_custom_model(id: i64, state: State<'_, AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    custom_model_settings::delete(&conn, id)
}

/// 上移/下移一位（调整编排优先级；已在顶/底部时静默成功）。
#[tauri::command]
pub fn settings_move_custom_model(
    id: i64,
    up: bool,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    custom_model_settings::move_by_one(&conn, id, up)?;
    Ok(())
}

/// 列表 inner（薄壳 → inner，便于 `#[test]` 直调，F1.2）。
fn list_custom_models_inner(conn: &Connection) -> Result<Vec<CustomModelSettingsDto>, AppError> {
    Ok(custom_model_settings::list_all(conn)?
        .into_iter()
        .map(|row| {
            // 先取借用型字段，再移动 String 字段（避免部分移动后借用）
            let models = row.models();
            let fingerprint = key_fingerprint(&row.api_key);
            let has_key = !row.api_key.is_empty();
            let enabled = row.is_enabled();
            CustomModelSettingsDto {
                id: row.id,
                name: row.name,
                base_url: row.base_url,
                models,
                has_key,
                key_fingerprint: fingerprint,
                enabled,
            }
        })
        .collect())
}

/// 保存 inner（薄壳 → inner，便于 `#[test]` 直调）。
fn save_custom_model_inner(
    conn: &Connection,
    id: Option<i64>,
    name: &str,
    base_url: &str,
    models: &[String],
    api_key: Option<&str>,
    enabled: bool,
) -> Result<i64, AppError> {
    // 薄层参数校验（空名/空 URL/空模型列表 → 配置错误，前端 toast 展示）
    if name.trim().is_empty() {
        return Err(AppError::Config("模型平台名称不能为空".into()));
    }
    if base_url.trim().is_empty() {
        return Err(AppError::Config("Base URL 不能为空".into()));
    }
    if models.iter().all(|m| m.trim().is_empty()) {
        return Err(AppError::Config("至少填写一个模型名称".into()));
    }
    let models: Vec<String> =
        models.iter().map(|m| m.trim().to_string()).filter(|m| !m.is_empty()).collect();
    let is_can_use = i64::from(enabled);

    match id {
        None => {
            // 新建：Key 缺省按空串落库（has_key=false，UI 提示补 Key）
            custom_model_settings::insert(
                conn,
                name.trim(),
                base_url.trim(),
                &models,
                api_key.unwrap_or(""),
            )
        },
        Some(id) => {
            custom_model_settings::update(
                conn,
                id,
                name.trim(),
                base_url.trim(),
                &models,
                is_can_use,
            )?;
            // Some（含空串=显式清空）= 改 Key；None = 不改动存量
            if let Some(key) = api_key {
                custom_model_settings::update_api_key(conn, id, key)?;
            }
            Ok(id)
        },
    }
}

/// Key 末 4 位指纹（脱敏口径与三档 `to_dto` 一致：单一真源）。
fn key_fingerprint(api_key: &str) -> Option<String> {
    if api_key.len() >= 4 { Some(api_key[api_key.len() - 4..].to_string()) } else { None }
}

/// 数据库锁错误统一映射（与既有命令层口径一致）。
fn db_lock_err<T: std::fmt::Display>(e: T) -> AppError {
    AppError::Db(format!("获取数据库连接失败: {e}"))
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
    fn settings_set_snap_sleep_roundtrip_via_snapshot() {
        let (_dir, conn) = crate::test_util::fresh_db();
        // 缺省 → 快照为 true（默认开启）
        let snap = settings_get_inner(&conn).expect("快照");
        assert!(snap.snap_sleep_enabled, "缺省应开启");

        set_snap_sleep_inner(&conn, false).expect("关闭");
        let snap = settings_get_inner(&conn).expect("快照");
        assert!(!snap.snap_sleep_enabled, "关闭后快照应为 false");

        set_snap_sleep_inner(&conn, true).expect("开启");
        let snap = settings_get_inner(&conn).expect("快照");
        assert!(snap.snap_sleep_enabled, "开启后快照应为 true");
    }

    #[test]
    fn settings_set_snap_ratio_persists_clamped_value() {
        use crate::db::repositories::app_settings;

        let (_dir, conn) = crate::test_util::fresh_db();
        // 缺省 → 0.5
        let snap = settings_get_inner(&conn).expect("快照");
        assert_eq!(snap.snap_show_ratio, SNAP_SHOW_RATIO_DEFAULT);

        // 合法值 → 原样落库
        let saved = set_snap_ratio_inner(&conn, 0.6).expect("写 0.6");
        assert_eq!(saved, 0.6);
        assert_eq!(app_settings::get_snap_show_ratio(&conn).expect("读取"), 0.6);

        // 越界 → clamp
        assert_eq!(set_snap_ratio_inner(&conn, 5.0).expect("写上界外"), SNAP_SHOW_RATIO_MAX);
        assert_eq!(set_snap_ratio_inner(&conn, -1.0).expect("写下界外"), SNAP_SHOW_RATIO_MIN);
    }

    #[test]
    fn settings_set_snap_ratio_roundtrip_and_clamped_via_snapshot() {
        let (_dir, conn) = crate::test_util::fresh_db();
        // 缺省 → 0.5（半个圆）
        let snap = settings_get_inner(&conn).expect("快照");
        assert_eq!(snap.snap_show_ratio, 0.5, "缺省应为半个圆");

        set_snap_ratio_inner(&conn, 0.7).expect("写 0.7");
        let snap = settings_get_inner(&conn).expect("快照");
        assert_eq!(snap.snap_show_ratio, 0.7, "应回读 0.7");

        set_snap_ratio_inner(&conn, 0.05).expect("写下界外");
        let snap = settings_get_inner(&conn).expect("快照");
        assert_eq!(snap.snap_show_ratio, 0.3, "超下界 → clamp 到 MIN");

        set_snap_ratio_inner(&conn, 9.9).expect("写上界外");
        let snap = settings_get_inner(&conn).expect("快照");
        assert_eq!(snap.snap_show_ratio, 0.8, "超上界 → clamp 到 MAX");
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

    // ---------- 自定义多模型管理 ----------

    #[test]
    fn save_custom_model_insert_then_update_semantics() {
        let (_dir, conn) = crate::test_util::fresh_db();
        let id = save_custom_model_inner(
            &conn,
            None,
            "我的 DeepSeek",
            "https://api.deepseek.com/v1",
            &["deepseek-chat".into()],
            Some("sk-abcd1234"),
            true,
        )
        .expect("新建");

        let list = list_custom_models_inner(&conn).expect("列表");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
        assert!(list[0].has_key, "Key 应落库");
        assert_eq!(list[0].key_fingerprint.as_deref(), Some("1234"), "仅末 4 位");
        assert!(list[0].enabled);

        // 更新：api_key=None → 不动存量；改 models/启用态
        save_custom_model_inner(
            &conn,
            Some(id),
            "改名",
            "https://api.deepseek.com/v1",
            &["a".into(), " b ".into()],
            None,
            false,
        )
        .expect("更新");
        let list = list_custom_models_inner(&conn).expect("列表");
        assert_eq!(list[0].name, "改名");
        assert_eq!(list[0].models, vec!["a", "b"], "空白项应被剔除并 trim");
        assert!(!list[0].enabled, "启用态应更新");
        assert!(list[0].has_key, "None = 不改动存量 Key");

        // 空串 Key = 显式清空
        save_custom_model_inner(
            &conn,
            Some(id),
            "改名",
            "https://api.deepseek.com/v1",
            &["a".into()],
            Some(""),
            true,
        )
        .expect("清空 Key");
        let list = list_custom_models_inner(&conn).expect("列表");
        assert!(!list[0].has_key, "空串 = 清空 Key");
        assert!(list[0].key_fingerprint.is_none());
    }

    #[test]
    fn save_custom_model_rejects_invalid_payloads() {
        let (_dir, conn) = crate::test_util::fresh_db();
        assert!(
            save_custom_model_inner(&conn, None, "  ", "https://x/v1", &["m".into()], None, true)
                .is_err(),
            "空名称应拒绝"
        );
        assert!(
            save_custom_model_inner(&conn, None, "n", "", &["m".into()], None, true).is_err(),
            "空 Base URL 应拒绝"
        );
        assert!(
            save_custom_model_inner(&conn, None, "n", "https://x/v1", &[], None, true).is_err(),
            "空模型列表应拒绝"
        );
        assert!(
            save_custom_model_inner(&conn, None, "n", "https://x/v1", &["  ".into()], None, true)
                .is_err(),
            "全空白模型应拒绝"
        );
    }

    #[test]
    fn custom_model_dto_never_leaks_plain_key() {
        let (_dir, conn) = crate::test_util::fresh_db();
        save_custom_model_inner(
            &conn,
            None,
            "泄漏测试",
            "https://api.example.com/v1",
            &["m".into()],
            Some("sk-SECRET-9876"),
            true,
        )
        .expect("新建");
        let list = list_custom_models_inner(&conn).expect("列表");
        let json = serde_json::to_string(&list).expect("序列化");
        assert!(!json.contains("sk-SECRET-9876"), "回包泄露明文 Key: {json}");
        assert!(!json.contains("SECRET"), "连 Key 主体亦不得出现: {json}");
        assert!(json.contains("9876"), "末 4 位指纹应存在");
    }
}

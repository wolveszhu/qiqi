//! 行结构体与命令 DTO（规范 §4.1）。
//!
//! 纪律：DTO 一律 `#[serde(rename_all = "camelCase")]`，与 `src/api/types.ts`
//! 逐个字段对齐；字段真源是 `docs/design/接口契约.md`，双端不得各自发明。

use serde::{Deserialize, Serialize};

// ---------- 行结构体（内部使用，不参与序列化） ----------

/// app_settings 行（`type` 是 Rust 关键字，字段改名 setting_type）。
pub struct AppSettingsRow {
    pub id: i64,
    pub setting_type: i64,
    pub is_custom_quota_used: i64,
    pub value: Option<String>,
}

/// pet_states 行（单用户单角色，id 固定 1）。
pub struct PetStatesRow {
    pub id: i64,
    pub pet_name: String,
    pub relationship_level: i64,
    pub affection_points: i64,
    pub last_decay_at: Option<String>,
    pub active_renderer: String,
    pub renderer_config: Option<String>,
    pub last_interaction: Option<String>,
}

/// facts 行。
pub struct FactRow {
    pub id: i64,
    pub category: i64,
    pub content: String,
    pub confidence: f64,
    pub created_at: String,
}

/// diaries 行。
pub struct DiaryRow {
    pub id: i64,
    pub diary_date: String,
    pub content: String,
    pub mood: Option<i64>,
    pub created_at: String,
}

/// affection_logs 行（好感度流水）。
pub struct AffectionLogRow {
    pub id: i64,
    pub change_value: i64,
    pub reason: String,
    pub created_at: String,
}

// ---------- app_settings.value 的存储结构 ----------

/// 单档 LLM 配置（库内存储形态；api_key 依赖 SQLCipher 全库加密，无需二次加密）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmTierValue {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model_custom: String,
}

/// 三档存储结构（type=1 行的 value）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TieredValue {
    #[serde(default)]
    pub main: LlmTierValue,
    #[serde(default)]
    pub memory: LlmTierValue,
    #[serde(default)]
    pub fallback: LlmTierValue,
}

/// 旧格式（M1 种子的扁平单档，snake_case 键）——兼容读取用，不用于写入。
#[derive(Debug, Deserialize)]
pub struct LegacyFlatValue {
    #[serde(default)]
    pub llm_base_url: String,
    #[serde(default)]
    pub llm_api_key: String,
    #[serde(default)]
    pub llm_model: String,
    #[serde(default)]
    pub llm_model_custom: String,
}

/// 勿扰时段区间。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DndRange {
    pub start: String,
    pub end: String,
}

/// 勿扰配置（type=2 行的 value.dnd，勘误#13 多段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DndValue {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub ranges: Vec<DndRange>,
}

/// type=2 行的完整 value（勘误 #13：dnd 多段 + 主动频率档位持久化于同一行）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDndValue {
    #[serde(default)]
    pub dnd: DndValue,
    /// 主动频率档位（"quiet" | "normal" | "sticky"；D2.3 持久化，C6 引擎读取）。
    #[serde(default)]
    pub frequency: String,
}

/// 旧勿扰格式（M1 种子单段，snake_case 键）——兼容读取用。
#[derive(Debug, Deserialize)]
pub struct LegacyDndValue {
    #[serde(default)]
    pub dnd_enabled: bool,
    #[serde(default)]
    pub dnd_start_time: String,
    #[serde(default)]
    pub dnd_end_time: String,
}

// ---------- 命令出参 DTO ----------

/// 单档配置的**对外视图**：api_key 绝不出现，只有 has_key + 末 4 位指纹。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmTierSettingsDto {
    pub base_url: String,
    pub model: String,
    pub has_key: bool,
    pub key_fingerprint: Option<String>,
    pub model_custom: String,
}

/// 设置快照（settings_get_typed 返回）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshotDto {
    pub main: LlmTierSettingsDto,
    pub memory: LlmTierSettingsDto,
    pub fallback: LlmTierSettingsDto,
    /// 1=内置额度 / 2=BYOK（app_settings.is_custom_quota_used）。
    pub is_custom_quota_used: i64,
    pub dnd: DndValue,
    /// 主动频率档位（D2.3：随 type=2 行一并下发，前端 settingsStore 消费）。
    pub frequency: String,
}

/// 桌宠状态（pet_get_state / pet_update_state 返回；好感度字段只读透出）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetStateDto {
    pub pet_name: String,
    pub relationship_level: i64,
    pub affection_points: i64,
    pub last_interaction: Option<String>,
}

/// 事实条目（memory_list_facts 返回）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactDto {
    pub id: i64,
    pub category: i64,
    pub content: String,
    pub confidence: f64,
    pub created_at: String,
}

/// 日记条目（memory_list_diaries 返回）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiaryDto {
    pub id: i64,
    pub diary_date: String,
    pub content: String,
    pub mood: Option<i64>,
}

/// 好感度流水条目（pet_list_affection_logs 返回，D2 看板）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectionLogDto {
    pub id: i64,
    pub change_value: i64,
    pub reason: String,
    pub created_at: String,
}

// ---------- 命令入参 DTO ----------

/// 保存单档配置（api_key 为空/缺省 = 不改动存量 Key）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTierPayload {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub model_custom: String,
}

/// 自定义模型对外视图（`custom_model_settings`；Key 脱敏，口径同 `LlmTierSettingsDto`）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModelSettingsDto {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    /// 模型名列表（空 = 该平台不参与编排，UI 应提示至少填一个）。
    pub models: Vec<String>,
    pub has_key: bool,
    pub key_fingerprint: Option<String>,
    /// 是否启用（停用不删，保配置）。
    pub enabled: bool,
}

/// 保存自定义模型入参（`id` 缺省 = 新建；`api_key` 缺省 = 不改动，空串 = 清空）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveCustomModelPayload {
    pub id: Option<i64>,
    pub name: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
}

/// 服务商预设（C3；经 IPC 下发，前端不复制列表）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPresetDto {
    pub id: String,
    pub label: String,
    pub base_url: String,
    /// 默认预填模型（可改；`modelCustom` 兜底）。
    pub default_model: String,
    pub required: bool,
    /// 成本档（展示用，不写死价格）。
    pub cost_tier: String,
}

/// 更新桌宠资料（好感度字段不接受修改——结算属 C5）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePetPayload {
    pub pet_name: Option<String>,
    pub active_renderer: Option<String>,
    pub renderer_config: Option<serde_json::Value>,
}

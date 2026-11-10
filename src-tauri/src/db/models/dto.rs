//! 命令层入参 / 出参 DTO。
//!
//! 纪律：一律 `#[serde(rename_all = "camelCase")]`，与 `src/api/types.ts`
//! 逐个字段对齐；字段真源是 `docs/design/接口契约.md`，双端不得各自发明。
//! 密钥类字段**只进不出**：入参可含 `api_key`，出参仅有 `has_key` + 指纹。

use serde::{Deserialize, Serialize};

use super::values::DndValue;

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
    /// 边缘吸附休眠开关（type=3 行；缺省 true，前端 AvatarPanel 消费）。
    pub snap_sleep_enabled: bool,
    /// 吸附态露出比例（type=3 行；占形象圆盘直径的比例，缺省 0.5=半个圆）。
    pub snap_show_ratio: f64,
}

/// 桌宠状态（pet_get_state / pet_update_state 返回；好感度字段只读透出）。
///
/// `active_renderer` 为当前生效的渲染器 id（`factory.ts` 注册表键，如 `code_cat` / `code_dog`），
/// 供前端读出，据此渲染用户选中的那套形象——此前该字段未出现在 DTO 里，
/// 导致形象选择从未被读回（重启后仍显示默认猫）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetStateDto {
    pub pet_name: String,
    pub relationship_level: i64,
    pub affection_points: i64,
    pub last_interaction: Option<String>,
    pub active_renderer: String,
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

/// 内置额度月度汇总（quota_summary 返回，设置面板展示用）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSummaryDto {
    /// 本地自然月周期键 YYYY-MM。
    pub period_key: String,
    /// 本月发放额度（credits）。
    pub granted: i64,
    /// 本月已用额度（credits）。
    pub used: i64,
    /// 剩余比例 0..=1（进度条）；granted<=0 时恒 0。
    pub remaining_ratio: f64,
    /// 本月记账请求次数。
    pub requests: i64,
    /// 按平台拆分的当月用量（used 降序；空月为空数组）。
    pub platforms: Vec<QuotaPlatformUsageDto>,
}

/// 单个平台的当月用量（quota_summary.platforms 元素）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaPlatformUsageDto {
    pub platform: String,
    pub used: i64,
    pub requests: i64,
}

/// 内置额度单次请求明细（quota_usage_records 返回，审计/对账用）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaUsageRecordDto {
    pub id: i64,
    pub created_at: String,
    pub period_key: String,
    /// 场景：chat / proactive / distill / diary。
    pub scene: String,
    /// 命中链段：builtin（记账）/ custom（不落库）。
    pub tier: String,
    /// 模型所属平台名（内置 = 智谱 / 硅基流动；无归属为 unknown）。
    pub platform: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    /// 计权后扣减点数（失败为 0）。
    pub credits: i64,
    pub ok: bool,
    /// 失败原因（成功为 None）。
    pub error: Option<String>,
}

/// 联网搜索服务配置（search_settings_get 返回；Key 脱敏）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSettingsDto {
    pub provider: String,
    pub has_key: bool,
    /// Key 末 4 位指纹（脱敏；无 Key 时为 None）。
    pub key_fingerprint: Option<String>,
    pub enabled: bool,
    pub max_results: i64,
    pub lang: String,
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

/// 服务商预设（C3；经 IPC 下发，前端不复制列表）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPresetDto {
    pub id: String,
    pub label: String,
    pub base_url: String,
    /// 默认预填模型（可改；`modelCustom` 兜底）。
    pub default_model: String,
    /// 常见模型列表（面板「+ 快速添加」数据源）。
    pub default_models: Vec<String>,
    pub required: bool,
    /// 成本档（展示用，不写死价格）。
    pub cost_tier: String,
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

/// 保存联网搜索配置（api_key 缺省 = 不改动存量 Key；空串 = 清空）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSearchSettingsPayload {
    pub provider: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub max_results: i64,
    pub lang: String,
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

/// 更新桌宠资料（好感度字段不接受修改——结算属 C5）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePetPayload {
    pub pet_name: Option<String>,
    pub active_renderer: Option<String>,
    pub renderer_config: Option<serde_json::Value>,
}

// ---------- 全局划词翻译（子计划：悬浮工具条与译文浮窗） ----------

/// 划词翻译配置对外视图（内置凭据与自定义 Key 均**不明文回传**）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateSettingsDto {
    pub enabled: bool,
    pub target_lang: String,
    pub provider: String,
    pub has_builtin_credential: bool,
    pub has_custom_key: bool,
    /// 自定义 Key 末 4 位指纹（无 Key 时为 None）。
    pub custom_key_fingerprint: Option<String>,
}

/// 保存划词翻译配置入参（凭据字段缺省 = 不改动存量；空串 = 清空）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTranslateSettingsPayload {
    pub enabled: bool,
    pub target_lang: String,
    pub provider: String,
    pub custom_api_key: Option<String>,
    pub custom_app_id: Option<String>,
}

/// 选区屏幕矩形（逻辑像素；多显示器 / DPI 换算在 Rust 侧完成）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRectDto {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// 取词结果（用户点击小图标后返回）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedSelectionDto {
    pub selection_key: String,
    pub text: String,
    pub rect: SelectionRectDto,
    /// 取词路径：`uia` 或 `clipboard`。
    pub via: String,
}

/// 译文结果（浮窗展示 + 重新翻译复用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResultDto {
    pub result_id: String,
    pub source_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub provider: String,
    /// 实际使用的服务名（自填服务商的展示名 / 内置「百度翻译」）。
    pub service_name: String,
    pub is_builtin: bool,
    pub quota_exhausted: bool,
}

/// `translate:selection:show` 事件负载。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateSelectionPayload {
    pub selection_key: String,
    pub rect: SelectionRectDto,
    pub cursor_x: i32,
    pub cursor_y: i32,
}

/// `translate:error` 事件负载（不含 Rust 中文 message，前端按 `kind` 本地化）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateErrorPayload {
    pub result_id: String,
    pub kind: String,
    pub retryable: bool,
}

/// 用户自填的翻译服务商（出参；Key 与 APPID 均不明文回传）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTranslateProviderDto {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub has_app_id: bool,
    pub has_key: bool,
    /// Key 末 4 位指纹（无 Key 时为 None）。
    pub key_fingerprint: Option<String>,
    pub enabled: bool,
}

/// 保存自填服务商入参（id 缺省 = 新建；api_key 缺省 = 不改动、空串 = 清空）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveCustomTranslateProviderPayload {
    pub id: Option<i64>,
    pub name: String,
    pub provider: String,
    /// 缺省 = 不改动存量（编辑时前端拿不到明文 APPID）。
    pub app_id: Option<String>,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
}

/// 内置翻译服务商对外视图（凭据**不明文回传**；含当月用量与各自上限）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinTranslateProviderDto {
    pub id: i64,
    pub provider_id: String,
    /// 展示名，同时是额度记账的 platform 键。
    pub name: String,
    pub enabled: bool,
    pub monthly_granted: i64,
    pub used: i64,
    pub requests: i64,
    pub has_credential: bool,
    /// 凭据末 4 位指纹（无凭据时为 None）。
    pub credential_fingerprint: Option<String>,
}

/// 保存内置服务商配置入参（只改启停与月度上限，凭据只读）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBuiltinTranslateProviderPayload {
    pub id: i64,
    pub enabled: bool,
    pub monthly_granted: i64,
}

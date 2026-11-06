//! SQL 行结构体：与查询结果逐列对应，**仅内部使用、不参与序列化**。
//!
//! 跨层传输请用 `dto` 中对应 DTO；含 `api_key` 的行（`CustomModelRow`、
//! `DefaultModelRow`）刻意就近定义在仓储文件内，避免明文行类型经本模块扩散。

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

/// quota_usage_monthly 行（内置额度月度汇总缓存）。
pub struct QuotaUsageMonthlyRow {
    pub period_key: String,
    pub granted: i64,
    pub used: i64,
    pub requests: i64,
}

/// quota_usage_platform_monthly 行（内置额度按平台月度汇总）。
pub struct QuotaPlatformMonthlyRow {
    pub period_key: String,
    pub platform: String,
    pub used: i64,
    pub requests: i64,
}

/// quota_usage 行（内置额度用量明细）。
pub struct QuotaUsageDetailRow {
    pub id: i64,
    pub created_at: String,
    pub period_key: String,
    pub scene: String,
    pub tier: String,
    /// 模型所属平台名（内置 = 智谱 / 硅基流动；无归属为 unknown）。
    pub platform: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub credits: i64,
    /// token 来源：api（接口 usage）/ estimated（端侧估算）。
    pub token_source: String,
    pub ok: bool,
    pub error: Option<String>,
}

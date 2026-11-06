//! `app_settings.value` 列的 JSON 存储结构（库内形态，**不是对外 DTO**）。
//!
//! `api_key` 依赖 SQLCipher 全库加密，无需二次加密；对外视图见 `dto`。
//! `Legacy*` 仅用于兼容读取历史库，绝不用于写入（M3 / M4 已把旧库迁到新结构）。

use serde::{Deserialize, Serialize};

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

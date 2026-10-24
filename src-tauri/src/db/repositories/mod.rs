//! 数据访问层（规范 §4.1）：**只写参数化 SQL，不做业务决策、不 emit 事件**。
//!
//! 红线（rule.md §H / code-standards §5）：
//! - 全部查询走 `params![]`，禁止字符串拼接式 SQL（DoD 以 grep 该目录无拼接宏为准）；
//! - 错误一律 `AppError::Db`，不做业务级 catch（错误由命令层原样上抛）。

pub mod affection_logs;
pub mod app_settings;
pub mod custom_model_settings;
pub mod default_model_settings;
pub mod diaries;
pub mod facts;
pub mod messages;
pub mod pet_states;
pub mod quota_usage;
pub mod summaries;
pub mod vec_memory;

use crate::error::AppError;

/// 解析 `models` JSON 数组为模型名列表（内置/自定义两张表共用的单一出口）。
///
/// 失败/非数组/元素非字符串 → 忽略该项；整体解析失败 → 空 Vec
/// （调用方按"无可用模型"跳过，不让坏数据炸掉整条编排链）。
pub(crate) fn parse_model_list(raw: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    value
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// 模型名数组序列化为库内 JSON 形态（写入侧单一出口，空列表 → `[]`）。
pub(crate) fn serialize_model_list(models: &[String]) -> Result<String, AppError> {
    serde_json::to_string(models).map_err(AppError::from)
}

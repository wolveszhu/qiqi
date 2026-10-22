//! 好感度流水 reason 串常量（C5：单一真源，避免散落字面量）。
//!
//! 与 `affection_logs.reason` 列对齐；规则引擎/命令层统一经此处取用。

/// 每日首次互动（打卡）。
pub const REASON_CHECK_IN: &str = "daily_check_in";
/// 抚摸。
pub const REASON_PET_STROKE: &str = "pet_stroke";
/// 深度对话。
pub const REASON_DEEP_CHAT: &str = "deep_chat";
/// 共鸣（C4 蒸馏管道判定）。
pub const REASON_RESONANCE: &str = "resonance";
/// 禁忌扣分（C7 判定）。
pub const REASON_TABOO: &str = "taboo_violation";
/// 长期未互动衰减（启动/懒结算写入）。
pub const REASON_DECAY: &str = "long_absence";

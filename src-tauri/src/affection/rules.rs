//! 加分规则引擎（C5.1）：纯函数，零 I/O，全程常量化（rule.md §4）。
//!
//! 信号 → 分值变更，全部可单测。阈值集中在文件顶部常量，无散落魔法数
//! （冷眼三问③）。reason 串由 `affection/mod.rs` 统一常量管理。

use super::reason::REASON_CHECK_IN;
use super::reason::REASON_DEEP_CHAT;
use super::reason::REASON_PET_STROKE;
use super::reason::REASON_RESONANCE;
use super::reason::REASON_TABOO;

/// 每日首次互动加分。
pub const CHECK_IN_POINTS: i64 = 10;
/// 单次抚摸加分。
pub const PET_STROKE_POINTS: i64 = 1;
/// 抚摸每日累计上限（change_value 合计超过即不再生效）。
pub const PET_STROKE_DAILY_CAP: i64 = 20;
/// 深度对话加分（MVP 量化：单轮回复字数 ≥ 阈值或单 session 轮次 ≥ 5）。
pub const DEEP_CHAT_POINTS: i64 = 5;
/// 共鸣强度 → 分值的映射表（index 即 strength；越界夹取）。
pub const RESONANCE_POINTS: [i64; 3] = [0, 10, 20];

/// 好感度信号（命令层/蒸馏管道经此枚举驱动规则引擎）。
///
/// - `CheckIn`：每日首次互动（PetWindow mount 触发）
/// - `PetStroke`：抚摸（窗口 A 双击/长按，C1 联动）
/// - `DeepChat`：深度对话（C4 done 携带字数/轮次调用）
/// - `Resonance(strength)`：共鸣（C4 蒸馏管道判定，strength 1/2）
/// - `Taboo(value)`：禁忌扣分（C7 判定，value 为负）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AffectionSignal {
    CheckIn,
    PetStroke,
    DeepChat,
    Resonance(u8),
    Taboo(i8),
}

/// 当日流水摘要（供 `apply` 判定唯一性/日上限；命令层从仓储取当日条目）。
#[derive(Debug, Clone)]
pub struct TodaysLog {
    pub reason: String,
    pub change_value: i64,
}

/// 规则引擎计算结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffectionChange {
    /// 本次净变更（正加负减）。
    pub delta: i64,
    /// 是否命中日上限（抚摸满 20 后不再涨）——供命令层日志/看板提示。
    pub capped: bool,
}

/// 计算信号对当前分数的净变更。
///
/// - 打卡：当日已有 `daily_check_in` → 0；否则 +10
/// - 抚摸：当日 pet_stroke 合计 < 20 → +1（满则 0，`capped=true`）
/// - 深度对话：+5（每 session 去重在命令层保证）
/// - 共鸣：strength 1→+10、2→+20、其它（含解析失败）→0（宁少不多）
/// - 禁忌：负值透传（不夹取下限，下限在结算/衰减层处理）
pub fn apply(points: i64, signal: &AffectionSignal, todays: &[TodaysLog]) -> AffectionChange {
    let _ = points; // 上限夹取由调用方在落库前统一 clamp，规则引擎只产出 delta
    match signal {
        AffectionSignal::CheckIn => {
            let already = todays.iter().any(|l| l.reason == REASON_CHECK_IN);
            if already {
                AffectionChange { delta: 0, capped: false }
            } else {
                AffectionChange { delta: CHECK_IN_POINTS, capped: false }
            }
        },
        AffectionSignal::PetStroke => {
            let sum: i64 = todays
                .iter()
                .filter(|l| l.reason == REASON_PET_STROKE)
                .map(|l| l.change_value)
                .sum();
            // 已达日上限（合计 >= 20）→ 不再涨，标记 capped；否则 +1（19→20 不标记）
            if sum >= PET_STROKE_DAILY_CAP {
                AffectionChange { delta: 0, capped: true }
            } else {
                AffectionChange { delta: PET_STROKE_POINTS, capped: false }
            }
        },
        AffectionSignal::DeepChat => AffectionChange { delta: DEEP_CHAT_POINTS, capped: false },
        AffectionSignal::Resonance(strength) => {
            let idx = (*strength as usize).min(RESONANCE_POINTS.len() - 1);
            // idx 已 clamp 到有效范围；兜底 0 防御（不应发生）
            AffectionChange {
                delta: RESONANCE_POINTS.get(idx).copied().unwrap_or(0),
                capped: false,
            }
        },
        AffectionSignal::Taboo(value) => AffectionChange { delta: *value as i64, capped: false },
    }
}

/// 信号 → 流水 reason 串（落库用，常量统一定义见 `mod_forward`）。
pub fn reason_of(signal: &AffectionSignal) -> &'static str {
    match signal {
        AffectionSignal::CheckIn => REASON_CHECK_IN,
        AffectionSignal::PetStroke => REASON_PET_STROKE,
        AffectionSignal::DeepChat => REASON_DEEP_CHAT,
        AffectionSignal::Resonance(_) => REASON_RESONANCE,
        AffectionSignal::Taboo(_) => REASON_TABOO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logs(pairs: &[(&str, i64)]) -> Vec<TodaysLog> {
        pairs.iter().map(|(r, v)| TodaysLog { reason: r.to_string(), change_value: *v }).collect()
    }

    #[test]
    fn check_in_is_unique_per_day() {
        // 无记录 → +10
        assert_eq!(apply(0, &AffectionSignal::CheckIn, &[]).delta, CHECK_IN_POINTS);
        // 已有打卡 → 0
        let today = logs(&[(REASON_CHECK_IN, 10)]);
        let c = apply(10, &AffectionSignal::CheckIn, &today);
        assert_eq!(c.delta, 0, "当日已有打卡不重复加");
    }

    #[test]
    fn pet_stroke_respects_daily_cap() {
        // 当日 19 → +1 到 20
        let today = logs(&[(REASON_PET_STROKE, 19)]);
        let c = apply(19, &AffectionSignal::PetStroke, &today);
        assert_eq!(c.delta, 1);
        assert!(!c.capped, "19→20 未达上限标记");
        // 当日 20 → 0 且 capped
        let today = logs(&[(REASON_PET_STROKE, 20)]);
        let c = apply(20, &AffectionSignal::PetStroke, &today);
        assert_eq!(c.delta, 0);
        assert!(c.capped, "已达日上限标记 capped");
    }

    #[test]
    fn resonance_maps_strength_and_falls_back_to_zero() {
        assert_eq!(apply(0, &AffectionSignal::Resonance(1), &[]).delta, 10);
        assert_eq!(apply(0, &AffectionSignal::Resonance(2), &[]).delta, 20);
        // strength 0 / 越界 / 解析失败（以 0 表达）→ 0（宁少不多）
        assert_eq!(apply(0, &AffectionSignal::Resonance(0), &[]).delta, 0);
        assert_eq!(apply(0, &AffectionSignal::Resonance(9), &[]).delta, 20, "越界夹取到 2");
    }

    #[test]
    fn deep_chat_and_taboo() {
        assert_eq!(apply(0, &AffectionSignal::DeepChat, &[]).delta, DEEP_CHAT_POINTS);
        assert_eq!(apply(100, &AffectionSignal::Taboo(-15), &[]).delta, -15, "禁忌负值透传");
    }
}

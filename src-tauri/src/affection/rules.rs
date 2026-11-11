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
/// 深度对话每日累计上限（**新增难度杠杆**：此前无上限，可狂聊刷分）。
pub const DEEP_CHAT_DAILY_CAP: i64 = 30;
/// 共鸣每日累计上限（**新增难度杠杆**：此前无上限）。
pub const RESONANCE_DAILY_CAP: i64 = 60;
/// 共鸣强度 → 分值的映射表（index 即 strength；越界夹取）。
pub const RESONANCE_POINTS: [i64; 3] = [0, 10, 20];

/// 收益递减权重表：按当日同类动作**已发生次数**（0-based）取权重折算本次得分。
///
/// 意图（用户裁决）：同一动作当天重复越多，单次加分越少，杜绝"机械重复刷分"。
/// 分数以整数入库，故用权重折算后**向下取整**（宁少不多）。
/// 表末 0.0 表示第 6 次起不再加分（与日上限共同封顶，双保险）。
pub const DIMINISH_WEIGHT: [f64; 6] = [1.0, 1.0, 0.6, 0.4, 0.2, 0.0];

/// 好感度信号（命令层/蒸馏管道经此枚举驱动规则引擎）。
///
/// - `CheckIn`：每日首次互动（PetWindow mount 触发）
/// - `PetStroke`：抚摸（窗口 A 双击/长按，C1 联动；日上限 20）
/// - `DeepChat`：深度对话（C4 done 携带字数调用；**日上限 30 + 收益递减**）
/// - `Resonance(strength)`：共鸣（C4 蒸馏管道判定，strength 1/2；**日上限 60 + 收益递减**）
/// - `Taboo(value)`：禁忌扣分（C7 判定，value 为负；不受递减影响，违规必扣）
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
/// - 深度对话：当日合计 < 30 → `+5 × 递减权重`（满则 0 且 `capped=true`）
/// - 共鸣：当日合计 < 60 → `+10/+20 × 递减权重`（满则 0 且 `capped=true`）
/// - 禁忌：负值透传（不夹取下限，下限在结算/衰减层处理）
///
/// **难度设计（用户裁决，反转 F3/F4「无上限」结论）**：深度对话与共鸣此前**无日上限**，
/// 是"涨分太容易"的主因（狂聊即可刷分）。现由「日上限 + 收益递减」双重封顶：
/// 递减让重复动作单次收益快速衰减，日上限兜住极端情况。抚摸沿用既有日上限 20
/// （本就是重复收益的封顶，不另加递减表，避免分数小数化复杂度）。
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
            let (sum, _) = reason_tally(todays, REASON_PET_STROKE);
            // 已达日上限（合计 >= 20）→ 不再涨，标记 capped；否则 +1（19→20 不标记）
            if sum >= PET_STROKE_DAILY_CAP {
                AffectionChange { delta: 0, capped: true }
            } else {
                AffectionChange { delta: PET_STROKE_POINTS, capped: false }
            }
        },
        AffectionSignal::DeepChat => {
            let (sum, count) = reason_tally(todays, REASON_DEEP_CHAT);
            if sum >= DEEP_CHAT_DAILY_CAP {
                return AffectionChange { delta: 0, capped: true };
            }
            // 递减到 0 同样标记为"今日不再加分"，供看板提示（非仅日上限触发）
            let delta = diminish(DEEP_CHAT_POINTS, count);
            AffectionChange { delta, capped: delta == 0 }
        },
        AffectionSignal::Resonance(strength) => {
            let (sum, count) = reason_tally(todays, REASON_RESONANCE);
            if sum >= RESONANCE_DAILY_CAP {
                return AffectionChange { delta: 0, capped: true };
            }
            let idx = (*strength as usize).min(RESONANCE_POINTS.len() - 1);
            // idx 已 clamp 到有效范围；兜底 0 防御（不应发生）
            let base = RESONANCE_POINTS.get(idx).copied().unwrap_or(0);
            let delta = diminish(base, count);
            AffectionChange { delta, capped: delta == 0 }
        },
        AffectionSignal::Taboo(value) => AffectionChange { delta: *value as i64, capped: false },
    }
}

/// 当日同类流水的（合计分值、已发生次数）——日上限判定与递减调度共用。
fn reason_tally(todays: &[TodaysLog], reason: &str) -> (i64, usize) {
    let mut sum = 0i64;
    let mut count = 0usize;
    for l in todays.iter().filter(|l| l.reason == reason) {
        sum += l.change_value;
        count += 1;
    }
    (sum, count)
}

/// 收益递减折算：按当日同类**已发生次数**取 `DIMINISH_WEIGHT` 权重，向下取整。
///
/// `occurred = 0` 即当日首次（权重最高，满分）；越往后越接近 0，表尾恒为 0。
/// 分数以整数入库，故 `(base × w).floor()`（宁少不多）；越界索引一律取 0。
fn diminish(base: i64, occurred: usize) -> i64 {
    let w = DIMINISH_WEIGHT.get(occurred).copied().unwrap_or(0.0);
    if w <= 0.0 {
        return 0;
    }
    (base as f64 * w).floor() as i64
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

    #[test]
    fn deep_chat_diminishes_and_caps_daily() {
        // 难度杠杆钉子：深度对话**有日上限 30 + 收益递减**（反转 F3/F4 无上限结论）。
        // 递减轨迹（+5 为基数）：5 / 5 / 3 / 2 / 1 / 0
        assert_eq!(apply(0, &AffectionSignal::DeepChat, &[]).delta, 5, "首轮满分");
        let one = logs(&[(REASON_DEEP_CHAT, 5)]);
        assert_eq!(apply(5, &AffectionSignal::DeepChat, &one).delta, 5, "第 2 轮仍满分");
        let two = logs(&[(REASON_DEEP_CHAT, 5), (REASON_DEEP_CHAT, 5)]);
        assert_eq!(apply(10, &AffectionSignal::DeepChat, &two).delta, 3, "第 3 轮降到 3");
        let four = logs(&[
            (REASON_DEEP_CHAT, 5),
            (REASON_DEEP_CHAT, 5),
            (REASON_DEEP_CHAT, 3),
            (REASON_DEEP_CHAT, 2),
        ]);
        assert_eq!(apply(15, &AffectionSignal::DeepChat, &four).delta, 1, "第 5 轮降到 1");
        // 第 6 轮起递减到 0（并被标记为今日不再加分）
        let five = logs(&[
            (REASON_DEEP_CHAT, 5),
            (REASON_DEEP_CHAT, 5),
            (REASON_DEEP_CHAT, 3),
            (REASON_DEEP_CHAT, 2),
            (REASON_DEEP_CHAT, 1),
        ]);
        let c = apply(16, &AffectionSignal::DeepChat, &five);
        assert_eq!(c.delta, 0, "第 6 轮递减到 0");
        assert!(c.capped, "递减到 0 同样标记 capped（供看板提示今日不再加分）");
        // 日上限兜底（合计 >= 30 → 0 且 capped）
        let at_cap = logs(&[(REASON_DEEP_CHAT, 30)]);
        let c = apply(30, &AffectionSignal::DeepChat, &at_cap);
        assert_eq!(c.delta, 0);
        assert!(c.capped, "达日上限标记 capped");
    }

    #[test]
    fn resonance_diminishes_and_caps_daily() {
        // 共鸣同深度对话：日上限 60 + 收益递减（按强度取基数后再折算）
        assert_eq!(apply(0, &AffectionSignal::Resonance(2), &[]).delta, 20, "首次 strength2 满分");
        let one = logs(&[(REASON_RESONANCE, 20)]);
        assert_eq!(
            apply(20, &AffectionSignal::Resonance(1), &one).delta,
            10,
            "第 2 次 strength1 满分"
        );
        let two = logs(&[(REASON_RESONANCE, 20), (REASON_RESONANCE, 10)]);
        assert_eq!(apply(30, &AffectionSignal::Resonance(2), &two).delta, 12, "第 3 次降到 12");
        // 日上限兜底
        let at_cap = logs(&[(REASON_RESONANCE, 60)]);
        let c = apply(60, &AffectionSignal::Resonance(2), &at_cap);
        assert_eq!(c.delta, 0);
        assert!(c.capped, "达日上限标记 capped");
    }

    #[test]
    fn taboo_is_not_diminished() {
        // 红线：禁忌扣分必须**不受递减影响**——违规必扣，不能因当天已扣过而减免
        let past = logs(&[(REASON_TABOO, -10), (REASON_TABOO, -10)]);
        assert_eq!(apply(100, &AffectionSignal::Taboo(-20), &past).delta, -20);
    }
}

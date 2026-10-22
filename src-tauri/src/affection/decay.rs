//! 弹性衰减引擎（C5.2）：纯函数，零 I/O（rule.md §4）。
//!
//! 语义（勘误 #9 裁决 #5）：`last_interaction` = 未互动起点；
//! `last_decay_at` = 上次结算时间（防重复扣）。结算区间
//! `[max(last_decay_at, last_interaction + 3d), today)` 内每一天 -5，
//! 止于当前等级下限（不破档、不为负）。幂等：同锚点重复结算结果不变。

use chrono::NaiveDate;

/// 每日衰减分值。
pub const DECAY_PER_DAY: i64 = 5;
/// 未互动宽限期（天）：宽限期内不扣。
pub const DECAY_GRACE_DAYS: i64 = 3;

/// 结算衰减。
///
/// 返回 `(new_points, new_anchor, days_decayed)`：
/// - `new_anchor`：下次结算起点（恒为 `today`，写回 `last_decay_at`）
/// - `days_decayed`：实际结算天数（0 = 无衰减，用于调用方判断是否 emit）
pub fn settle_decay(
    points: i64,
    level_floor: i64,
    last_interaction: Option<NaiveDate>,
    last_decay_at: Option<NaiveDate>,
    today: NaiveDate,
) -> (i64, NaiveDate, i64) {
    // 起点：last_interaction + grace 天（无记录则视为今天）
    let start_from_li =
        last_interaction.map(|li| li + chrono::Duration::days(DECAY_GRACE_DAYS)).unwrap_or(today);
    // 取 max(上次结算, 起点)——已有结算记录则从其之后继续
    let decay_start = match last_decay_at {
        Some(d) => d.max(start_from_li),
        None => start_from_li,
    };

    let days = (today - decay_start).num_days();
    if days <= 0 {
        return (points, today, 0);
    }
    let new_points = (points - days * DECAY_PER_DAY).max(level_floor).max(0);
    (new_points, today, days)
}

#[cfg(test)]
mod tests {
    use super::super::level;
    use super::*;
    use chrono::NaiveDate;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn no_decay_within_grace_or_zero_days() {
        // 恰 3 天未互动 → 起点=今天 → 不扣
        let (np, _a, days) = settle_decay(
            100,
            level::level_floor(level::Level::L1),
            Some(d("2026-01-01")),
            None,
            d("2026-01-04"),
        );
        assert_eq!(days, 0);
        assert_eq!(np, 100);
        // last_interaction / last_decay_at 均缺失 → 不扣
        let (np, _a, days) = settle_decay(100, 0, None, None, d("2026-01-04"));
        assert_eq!(days, 0);
        assert_eq!(np, 100);
    }

    #[test]
    fn decay_scales_with_absence() {
        // 4 天 → 1 天衰减 → -5
        let (np, _, days) = settle_decay(100, 0, Some(d("2026-01-01")), None, d("2026-01-05"));
        assert_eq!(days, 1);
        assert_eq!(np, 95);
        // 10 天 → 7 天衰减 → -35
        let (np, _, days) = settle_decay(100, 0, Some(d("2026-01-01")), None, d("2026-01-11"));
        assert_eq!(days, 7);
        assert_eq!(np, 65);
    }

    #[test]
    fn decay_stops_at_level_floor() {
        // 500 分 Lv2（下限 201），挂机 100 天 → 精确停在 201
        let floor = level::level_floor(level::level_of(500));
        assert_eq!(floor, 201);
        let (np, _, days) = settle_decay(500, floor, Some(d("2026-01-01")), None, d("2026-04-11"));
        assert_eq!(days, 97, "100 天 - 3 天宽限 = 97 天结算");
        assert_eq!(np, 201, "止于档下限，不破档、不为负");
    }

    #[test]
    fn idempotent_same_anchor() {
        // 结算一次后 last_decay_at=today；立即再结算 → days=0 不变
        let (np1, anchor, _) = settle_decay(200, 201, Some(d("2026-01-01")), None, d("2026-02-01"));
        let (np2, _, days2) =
            settle_decay(np1, 201, Some(d("2026-01-01")), Some(anchor), d("2026-02-01"));
        assert_eq!(days2, 0);
        assert_eq!(np2, np1, "同锚点重复结算结果不变");
    }
}

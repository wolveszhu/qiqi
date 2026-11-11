//! 等级映射（C5.3）：纯函数，零 I/O（rule.md §4）。
//!
//! 区间（**梯度上升**，用户裁决）：Lv1 陌生 0–200 / Lv2 熟人 201–550 / Lv3 亲密 551–1050 /
//! Lv4 羁绊 1051–2000。每升一级所需间隔为 200 / 350 / 500（等差 +150，严格递增），
//! 越往上越难。等级→行为指令文案供 C4 `prompt.rs` 注入（单一真源）。

/// 好感度上限（封顶）。
pub const MAX_POINTS: i64 = 2000;

/// 等级起始阈值（索引即等级-1；与 `level_of` 边界单一真源对齐）。
pub const LEVEL_FLOORS: [i64; 4] = [0, 201, 551, 1051];

/// 等级（1..=4，与 `pet_states.relationship_level` 存储一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    L1,
    L2,
    L3,
    L4,
}

/// 分数 → 等级（边界全覆盖：200→L1、201→L2、550→L2、551→L3、1050→L3、1051→L4）。
///
/// 下界写 `..=200` 而非 `0..=200`：防御负分被误判为 L4（分数理论上已被调用方 clamp，
/// 但此处兜底更稳——负分归最低档而非最高档）。
pub fn level_of(points: i64) -> Level {
    match points {
        ..=200 => Level::L1,
        201..=550 => Level::L2,
        551..=1050 => Level::L3,
        _ => Level::L4,
    }
}

/// 等级 → 存储索引（1..=4）。
pub fn level_index(level: Level) -> i64 {
    match level {
        Level::L1 => 1,
        Level::L2 => 2,
        Level::L3 => 3,
        Level::L4 => 4,
    }
}

/// 等级 → 衰减止损下限（降至当前档下限即停；L1 下限为 0）。
///
/// 与 `level_of` 边界同源：`floor(Ln) = 边界[Ln-1] + 1`（由 `LEVEL_FLOORS` 单一真源提供）。
pub fn level_floor(level: Level) -> i64 {
    let idx = (level_index(level) - 1) as usize;
    // `level_index` 恒返回 1..=4 → idx 0..=3，与 4 元数组等长；`.get` 仅作防御取用
    LEVEL_FLOORS.get(idx).copied().unwrap_or(0)
}

/// 升级/降级检测（同档返回 None；跨档返回方向）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelShift {
    Up,
    Down,
    None,
}

pub fn detect_shift(old: Level, new: Level) -> LevelShift {
    let o = level_index(old);
    let n = level_index(new);
    if n > o {
        LevelShift::Up
    } else if n < o {
        LevelShift::Down
    } else {
        LevelShift::None
    }
}

/// 等级 → 行为指令文案（C4 prompt 注入；单一真源，避免与 config 重复定义）。
///
/// 直接复用 `config::RELATION_LEVEL_DIRECTIVES`（四档常量），按等级索引取用。
pub fn behavior_prompt(level: Level) -> &'static str {
    let idx = (level_index(level) - 1) as usize;
    // 等级索引恒在 0..=3（level_index 返回 1..=4）；越界兜底空串（不应发生）
    let directives = crate::config::RELATION_LEVEL_DIRECTIVES;
    debug_assert!(idx < directives.len());
    directives.get(idx).copied().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_boundaries() {
        assert_eq!(level_of(0), Level::L1);
        assert_eq!(level_of(200), Level::L1);
        assert_eq!(level_of(201), Level::L2);
        assert_eq!(level_of(550), Level::L2);
        assert_eq!(level_of(551), Level::L3);
        assert_eq!(level_of(1050), Level::L3);
        assert_eq!(level_of(1051), Level::L4);
        assert_eq!(level_of(MAX_POINTS), Level::L4);
        assert_eq!(level_of(-5), Level::L1, "负分兜底归最低档，不得落入 L4");
    }

    #[test]
    fn level_gaps_increase_strictly() {
        // 用户裁决钉子：每升一级所需分数**梯度上升**（跨度 201 / 350 / 500）。
        // 若有人改回等距或倒挂，此测试会失败——强制同步文档与前端阈值表。
        let gaps: Vec<i64> =
            (1..LEVEL_FLOORS.len()).map(|i| LEVEL_FLOORS[i] - LEVEL_FLOORS[i - 1]).collect();
        assert_eq!(gaps, vec![201, 350, 500], "各档跨度：L1 0–200、L2 350、L3 500");
        assert!(gaps.windows(2).all(|w| w[1] > w[0]), "每升一级所需分数必须严格递增");
    }

    #[test]
    fn floor_and_index() {
        assert_eq!(level_floor(Level::L1), 0);
        assert_eq!(level_floor(Level::L2), 201);
        assert_eq!(level_floor(Level::L3), 551);
        assert_eq!(level_floor(Level::L4), 1051);
        assert_eq!(level_index(Level::L4), 4);
        assert_eq!(level_index(Level::L1), 1);
    }

    #[test]
    fn shift_detection() {
        assert_eq!(detect_shift(Level::L1, Level::L2), LevelShift::Up);
        assert_eq!(detect_shift(Level::L2, Level::L1), LevelShift::Down);
        assert_eq!(detect_shift(Level::L2, Level::L2), LevelShift::None);
        // 非相邻跨档同样能识别
        assert_eq!(detect_shift(Level::L1, Level::L4), LevelShift::Up);
    }

    #[test]
    fn behavior_prompt_indexed_by_level() {
        assert!(behavior_prompt(Level::L1).contains("刚认识"));
        assert!(behavior_prompt(Level::L4).contains("无可替代"));
    }
}

//! 等级映射（C5.3）：纯函数，零 I/O（rule.md §4）。
//!
//! 区间（baseline §F）：Lv1 陌生 0–200 / Lv2 熟人 201–500 / Lv3 亲密 501–800 /
//! Lv4 羁绊 801–1000。等级→行为指令文案供 C4 `prompt.rs` 注入（单一真源）。

/// 好感度上限（封顶）。
pub const MAX_POINTS: i64 = 1000;

/// 等级（1..=4，与 `pet_states.relationship_level` 存储一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    L1,
    L2,
    L3,
    L4,
}

/// 分数 → 等级（边界全覆盖：200→L1、201→L2、500→L2、501→L3、800→L3、801→L4）。
pub fn level_of(points: i64) -> Level {
    match points {
        0..=200 => Level::L1,
        201..=500 => Level::L2,
        501..=800 => Level::L3,
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
pub fn level_floor(level: Level) -> i64 {
    match level {
        Level::L1 => 0,
        Level::L2 => 201,
        Level::L3 => 501,
        Level::L4 => 801,
    }
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
    crate::config::RELATION_LEVEL_DIRECTIVES[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_boundaries() {
        assert_eq!(level_of(0), Level::L1);
        assert_eq!(level_of(199), Level::L1);
        assert_eq!(level_of(200), Level::L1);
        assert_eq!(level_of(201), Level::L2);
        assert_eq!(level_of(500), Level::L2);
        assert_eq!(level_of(501), Level::L3);
        assert_eq!(level_of(800), Level::L3);
        assert_eq!(level_of(801), Level::L4);
        assert_eq!(level_of(1000), Level::L4);
    }

    #[test]
    fn floor_and_index() {
        assert_eq!(level_floor(Level::L1), 0);
        assert_eq!(level_floor(Level::L2), 201);
        assert_eq!(level_floor(Level::L3), 501);
        assert_eq!(level_floor(Level::L4), 801);
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

/**
 * 好感度**展示**映射（D2 / 设计方案 §4.4）：等级阈值、等级文案、圆环进度。
 *
 * 纯函数单一真源——`LevelPill` / `AffectionPanel` / `AffectionRing` 共用，
 * 边界值由 `tests/utils/affection.spec.ts` 固化（0 / 200 / 550 / 1050 / 2000）。
 *
 * ⚠️ 与 Rust C5 的边界：规则引擎与分值结算在 Rust（`affection/level.rs` 的 `LEVEL_FLOORS`
 * 是等级**起始**阈值 0/201/551/1051；此处的 `LEVEL_THRESHOLDS` 用各档**上沿**
 * 0/200/550/1050 表示"距本档顶还差多少"，与 Rust 共享同一组真实档位，
 * 仅末位哨兵为 `AFFECTION_MAX`）。窗口 A 对话气泡绝不展示数值（红线见 E3 / prompt.rs）。
 */

/** 等级名（1–4；索引 0 留空占位）。 */
export const LEVEL_NAMES = ["", "陌生", "熟人", "亲密", "羁绊"] as const;

/** 等级 → 文案（0 占位/越界一律回退 `Lv.N`；实际等级恒 1–4，此处为防御）。 */
export function levelLabel(level: number): string {
  const name = LEVEL_NAMES[level];
  return name ? name : `Lv.${level}`;
}

/**
 * 等级阈值（展示用，**与 Rust `level.rs` 档位对齐**，修掉此前四等分近似的展示 bug）。
 *
 * 索引 = 当前等级，值 = 该档**上沿**（"距本档顶还差多少分"的目标）：
 * L1 0–200 / L2 201–550 / L3 551–1050 / L4 1051–2000。
 * 末位 2000 仅作 `nextLevelInfo` 越界哨兵（实际 level≥4 已短路）。
 */
export const LEVEL_THRESHOLDS = [0, 200, 550, 1050, 2000] as const;

/** 下档表现文案（情绪化，非数值；索引 0/5 留空占位）。 */
export const LEVEL_SUB = [
  "",
  "刚认识，有点拘谨",
  "熟悉了，会主动搭话",
  "亲密，会撒娇会想念",
  "羁绊，形影不离",
] as const;

/** 下一档信息。 */
export interface NextLevelInfo {
  /** 距下一档还差多少分（已到顶为 0，绝不报负）。 */
  diff: number;
  /** 下一档/顶格情绪文案。 */
  sub: string;
}

/** 下一档信息（level ≥ 4 → 顶格；阈值越界兜底 AFFECTION_MAX）。 */
export function nextLevelInfo(level: number, points: number): NextLevelInfo {
  if (level >= 4) return { diff: 0, sub: "已抵达最高羁绊" };
  const next = LEVEL_THRESHOLDS[level] ?? AFFECTION_MAX;
  return {
    diff: Math.max(0, next - points),
    sub: LEVEL_SUB[level + 1] ?? "",
  };
}

/** 好感度上限（封顶 2000；与 Rust `level.rs::MAX_POINTS` 一致）。 */
export const AFFECTION_MAX = 2000;

/** 进度百分比（0–1；clamp 负值与超限值）。 */
export function ringPct(points: number): number {
  return Math.max(0, Math.min(AFFECTION_MAX, points)) / AFFECTION_MAX;
}

/** 圆环角度（deg，0–360；conic-gradient 用）。 */
export function ringAngle(points: number): number {
  return ringPct(points) * 360;
}

/** 流水 reason 的展示元数据。 */
export interface ReasonMeta {
  /** 中文展示文案（后端落库的是英文 key，此处翻译）。 */
  label: string;
  /** 条目前置 emoji。 */
  emoji: string;
}

/**
 * reason（英文 key）→ 展示元数据。
 *
 * 后端 `affection/reason.rs` 落库的是英文 key（存量行亦然），展示层统一在此翻译；
 * 未知 key 回退「原样 + ✨」——不臆造文案，便于排查新接入的信号。
 */
const REASON_META: Record<string, ReasonMeta> = {
  daily_check_in: { label: "每日打卡", emoji: "🌞" },
  pet_stroke: { label: "抚摸", emoji: "🐾" },
  deep_chat: { label: "深度对话", emoji: "💬" },
  resonance: { label: "情感共鸣", emoji: "💬" },
  taboo_violation: { label: "触犯禁忌", emoji: "😾" },
  long_absence: { label: "久未互动衰减", emoji: "💤" },
};

/** reason → 展示元数据（未知 key 原样回退）。 */
export function reasonMeta(reason: string): ReasonMeta {
  return REASON_META[reason] ?? { label: reason, emoji: "✨" };
}

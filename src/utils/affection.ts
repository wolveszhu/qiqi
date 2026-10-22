/**
 * 好感度**展示**映射（D2 / 设计方案 §4.4）：等级阈值、等级文案、圆环进度。
 *
 * 纯函数单一真源——`LevelPill` / `AffectionPanel` / `AffectionRing` 共用，
 * 边界值由 `tests/utils/affection.spec.ts` 固化（0 / 200 / 201 / 500 / 750 / 1000）。
 *
 * ⚠️ 与 Rust C5 的边界：规则引擎与分值结算在 Rust（F1 已测），此处仅**展示层**近似
 * （窗口 B 好感度看板可见）；窗口 A 对话气泡绝不展示数值（红线见 E3 / prompt.rs）。
 */

/** 等级名（1–4；索引 0 留空占位）。 */
export const LEVEL_NAMES = ["", "陌生", "熟人", "亲密", "羁绊"] as const;

/** 等级 → 文案（0 占位/越界一律回退 `Lv.N`；实际等级恒 1–4，此处为防御）。 */
export function levelLabel(level: number): string {
  const name = LEVEL_NAMES[level];
  return name ? name : `Lv.${level}`;
}

/** 下一档阈值（展示近似：0–1000 四等分；与 C5 等级映射视觉对齐）。 */
export const LEVEL_THRESHOLDS = [0, 250, 500, 750, 1000] as const;

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

/** 下一档信息（level ≥ 4 → 顶格；阈值越界兜底 1000）。 */
export function nextLevelInfo(level: number, points: number): NextLevelInfo {
  if (level >= 4) return { diff: 0, sub: "已抵达最高羁绊" };
  const next = LEVEL_THRESHOLDS[level] ?? AFFECTION_MAX;
  return {
    diff: Math.max(0, next - points),
    sub: LEVEL_SUB[level + 1] ?? "",
  };
}

/** 好感度上限（0–1000）。 */
export const AFFECTION_MAX = 1000;

/** 进度百分比（0–1；clamp 负值与超限值）。 */
export function ringPct(points: number): number {
  return Math.max(0, Math.min(AFFECTION_MAX, points)) / AFFECTION_MAX;
}

/** 圆环角度（deg，0–360；conic-gradient 用）。 */
export function ringAngle(points: number): number {
  return ringPct(points) * 360;
}

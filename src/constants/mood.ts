/**
 * 日记情绪映射（D3 / 设计 §4.5）。
 *
 * 值域以后端为准（`llm/diary.rs` 契约）：`0=开心 / 1=平静 / 2=低落`，
 * `null`=未记录（LLM 未产出）。⚠️ 与 D3 子计划 §一#2 草稿的 `0-4` 映射冲突，
 * 以后端落地契约为真源（子计划为陈旧草稿，已登记于 D3 回写）。
 */

export type DiaryMood = 0 | 1 | 2;

export interface MoodView {
  emoji: string;
  label: string;
}

/** mood → 展示（emoji + 文案）。 */
export const MOOD_VIEW: Record<DiaryMood, MoodView> = {
  0: { emoji: "😺", label: "开心" },
  1: { emoji: "😌", label: "平静" },
  2: { emoji: "😔", label: "低落" },
};

/** 统一入口：null/越界 → 平静兜底。 */
export function moodView(m: DiaryMood | null | undefined): MoodView {
  if (m == null) return { emoji: "😌", label: "平静" };
  return MOOD_VIEW[m] ?? { emoji: "😌", label: "平静" };
}

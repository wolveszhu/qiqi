/**
 * 好感度展示文案的本地化适配层（窗口 B 专用）。
 *
 * `utils/affection` 是纯函数单一真源（含 F2 单测固化中文原值与数值边界），
 * 不宜掺入 i18n 依赖；此处仅在展示层把等级名 / 流水原因 / 距下档情绪文案
 * 映射为 vue-i18n 键，未知值回落纯函数结果，避免新等级或新 reason 丢失文案。
 */

import { useI18n } from "vue-i18n";

import { levelLabel, nextLevelInfo, reasonMeta } from "../utils/affection";

/** 随语言切换的好感度展示文案函数集合。 */
export interface AffectionI18n {
  /** 等级名（1–4 走字典；越界回退 `Lv.N`）。 */
  levelName: (level: number) => string;
  /** 流水原因本地化文案。 */
  reasonLabel: (reason: string) => string;
  /** 距下一档的情绪文案。 */
  nextSub: (level: number, points: number) => string;
}

/** 返回随语言切换的好感度展示文案函数。 */
export function useAffectionI18n(): AffectionI18n {
  const { t, te } = useI18n();

  /** 等级名（1–4 走字典；越界回退 `Lv.N`）。 */
  function levelName(level: number): string {
    const key = `level.l${level}`;
    return te(key) ? t(key) : levelLabel(level);
  }

  /** 流水原因文案（后端英文 key；未知 key 回退原样 + ✨ 语义）。 */
  function reasonLabel(reason: string): string {
    const key = `reason.${reason}`;
    return te(key) ? t(key) : reasonMeta(reason).label;
  }

  /** 距下一档的情绪文案（diff 仍由纯函数计算，仅文案本地化）。 */
  function nextSub(level: number, points: number): string {
    const info = nextLevelInfo(level, points);
    if (level >= 4) return t("levelSub.max");
    const key = `levelSub.l${level + 1}`;
    return te(key) ? t(key) : info.sub;
  }

  return { levelName, reasonLabel, nextSub };
}

/**
 * 气泡节奏策略（C6.4 / E3.3）：打字机出字节奏 + 自动消失时长。
 *
 * 纯函数、零依赖（不拉 PIXI/Tauri/状态）——`useProactive` 编排引用，
 * `tests/utils/proactiveRhythm.spec.ts` 直测。
 */

/** 打字机出字节奏（ms/步）。 */
export const TYPEWRITER_STEP_MS = 40;
/** 每步出字数（长句加速，保证 <8s 完整展示）。 */
export const TYPEWRITER_CHUNK = 2;

/** 停留下限（ms）。 */
const TTL_MIN_MS = 2_000;
/** 停留上限（ms）。 */
const TTL_MAX_MS = 12_000;
/** 每字递增（ms/字）。 */
const TTL_PER_CHAR_MS = 50;
/** 禁忌/傲娇情绪话术停留倍率。 */
const EMOTION_TTL_FACTOR = 1.5;

/**
 * 气泡自动消失时长（毫秒）：按字数（min 2s / +0.05s·字，上限 12s）；
 * 禁忌(angry)/傲娇(tsundere)情绪话术停留加长 1.5×（E3.3）。
 */
export function bubbleTtlMs(text: string, mood: string): number {
  const chars = [...text].length;
  const base = Math.min(
    TTL_MAX_MS,
    Math.max(TTL_MIN_MS, TTL_MIN_MS + chars * TTL_PER_CHAR_MS),
  );
  if (mood === "angry" || mood === "tsundere") {
    return Math.min(TTL_MAX_MS, Math.round(base * EMOTION_TTL_FACTOR));
  }
  return base;
}

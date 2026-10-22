/**
 * 微随机纯函数引擎（C6.3，裁决 §一 #1：仲裁逻辑全在前端）。
 *
 * 纪律：**全部注入 rng/now 可测**（F2 覆盖项）——零全局状态、零 I/O；
 * `shouldFire` 内部按"勿扰 → 最小间隔 → 日预算 → 概率"四重约束判定。
 */

import {
  ANCHOR_FIRE_PROBABILITY,
  ANCHOR_WINDOWS,
  BASE_FIRE_PROBABILITY,
  MIN_FIRE_INTERVAL_MS,
  TIER_BUDGET,
  TSUNDERE_DAYS,
  WORK_PROCESS_MULTIPLIER,
  WORK_PROCESS_NAMES,
  type AnchorWindow,
} from "../constants/proactive";
import type { ProactiveFrequency } from "../api/types";

/** 随机源（注入：Math.random 或确定性桩）。返回 [0,1)。 */
export type Rng = () => number;

/** 触发判定上下文（编排层组装；now/rng 外部注入）。 */
export interface ProactiveCtx {
  /** 当前毫秒时间戳（注入，测试可控）。 */
  now: number;
  /** 主动频率档位。 */
  tier: ProactiveFrequency;
  /** 勿扰总开关（D2 配置）。 */
  dndEnabled: boolean;
  /** 当前是否处于勿扰时段内（编排层算好，保持本文件零配置解析）。 */
  inDndRange: boolean;
  /** 前台进程名（探针快照；null=无前台）。 */
  foreground: string | null;
  /** 当日已触发次数（编排层计数）。 */
  budgetUsed: number;
  /** 上次触发时间戳（null=当日未触发过）。 */
  lastFireAt: number | null;
  /** 锚点窗口内但当日已触发过 → 不再强制高概率（锚点当日去重，裁决 #3）。 */
  anchorBoostSuppressed?: boolean;
}

/** 档位日预算：区间内均匀抽取（含两端；rng 注入确定性）。 */
export function dailyBudget(tier: ProactiveFrequency, rng: Rng): number {
  const [lo, hi] = TIER_BUDGET[tier];
  return lo + Math.floor(rng() * (hi - lo + 1));
}

/** 当前时刻在锚点窗口内（含起止边界；now 注入）。 */
export function inAnchorWindow(
  now: number,
  anchors: AnchorWindow[] = ANCHOR_WINDOWS,
): boolean {
  const d = new Date(now);
  const minutes = d.getHours() * 60 + d.getMinutes();
  return anchors.some((a) => minutes >= a.startMin && minutes <= a.endMin);
}

/** 傲娇判定：距上次互动 ≥ 3 天（72h 边界；无记录 → false）。 */
export function isTsundere(lastInteractionMs: number | null, now: number): boolean {
  if (lastInteractionMs == null) return false;
  return now - lastInteractionMs >= TSUNDERE_DAYS * 24 * 60 * 60 * 1000;
}

/** 工作/游戏进程匹配（进程名包含匹配，小写；null → 非工作场景）。 */
export function matchWorkProcess(name: string | null): boolean {
  if (!name) return false;
  const lower = name.toLowerCase();
  return WORK_PROCESS_NAMES.some((w) => lower.includes(w.toLowerCase()));
}

/** 综合判定本 tick 是否主动开口（勿拒/间隔/预算/概率；rng 注入确定性）。 */
export function shouldFire(ctx: ProactiveCtx, rng: Rng): boolean {
  // ① 勿扰优先（穿透总闸联动 D4：全程静默）
  if (ctx.dndEnabled && ctx.inDndRange) return false;
  // ② 最小间隔（防刷屏）
  if (ctx.lastFireAt != null && ctx.now - ctx.lastFireAt < MIN_FIRE_INTERVAL_MS)
    return false;
  // ③ 当日预算（区间内抽取；用尽即静默）
  if (ctx.budgetUsed >= dailyBudget(ctx.tier, rng)) return false;
  // ④ 概率：工作/游戏 ×0.2；锚点窗口强制高概率（当日已触发过则抑制）
  let probability = BASE_FIRE_PROBABILITY;
  if (matchWorkProcess(ctx.foreground)) probability *= WORK_PROCESS_MULTIPLIER;
  if (inAnchorWindow(ctx.now) && !ctx.anchorBoostSuppressed) {
    probability = Math.max(probability, ANCHOR_FIRE_PROBABILITY);
  }
  return rng() < probability;
}

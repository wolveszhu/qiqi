/**
 * 主动互动编排（C6.4）：5min tick → 微随机仲裁 → Rust 短生成 → 气泡打字机。
 *
 * 数据流（计划 §三）：
 *   tick（纯计算零 I/O）
 *     ├─ 勿扰？→ skip（shouldFire 内最高优先）
 *     ├─ shouldFire(rng 注入)：预算/间隔/降频/锚点（锚点当日去重经
 *     │   anchorBoostSuppressed 抑制高概率，裁决 #3）
 *     └─ 命中 → proactiveGenerate(tsundere) → Rust C4 管道
 *          └─ proactive:show 事件 → 本地逐字打字机 + 表情联动
 *
 * 性能红线：tick 纯计算；打字机单定时器；卸载全量清理（冷眼三问①）。
 * 当日锚点去重/预算计数为**内存态**（裁决 §一 #3：重启后锚点窗口内可能
 * 重复一次，窗口仅 1h，接受；不加 DB 状态）。
 */

import { onMounted, onUnmounted, reactive } from "vue";

import { onProactiveShow, proactiveGenerate } from "../api/probe";
import { getState } from "../api/pet";
import {
  ANCHOR_WINDOWS,
  MIN_FIRE_INTERVAL_MS,
  PROACTIVE_TICK_MS,
} from "../constants/proactive";
import { applyPetState, petStore } from "../state/petStore";
import { probeStore } from "../state/probeStore";
import { settingsStore } from "../state/settingsStore";
import { setExpression } from "./usePetRenderer";
import { isTsundere, shouldFire, type ProactiveCtx } from "../utils/microRandom";
import type { ProactiveShowPayload } from "../api/types";
import type { UnlistenFn } from "@tauri-apps/api/event";

/** 主动气泡状态（PetWindow 渲染；单气泡覆盖式，勘误#12 同口径）。 */
export const proactiveBubble = reactive<{ text: string; isStreaming: boolean }>({
  text: "",
  isStreaming: false,
});

/** 打字机出字节奏（ms/步）。 */
const TYPEWRITER_STEP_MS = 40;
/** 每步出字数（长句加速，保证 <8s 完整展示）。 */
const TYPEWRITER_CHUNK = 2;
/** 气泡自动消失时长（毫秒）：按字数（min 2s / +0.05s·字，上限 12s）；禁忌/傲娇情绪话术停留加长 1.5×（E3.3）。 */
function bubbleTtlMs(text: string, mood: string): number {
  const chars = [...text].length;
  const base = Math.min(12_000, Math.max(2_000, 2_000 + chars * 50));
  if (mood === "angry" || mood === "tsundere")
    return Math.min(12_000, Math.round(base * 1.5));
  return base;
}

const unlisteners: UnlistenFn[] = [];
let tickTimer: ReturnType<typeof setInterval> | null = null;
let typeTimer: ReturnType<typeof setInterval> | null = null;
let ttlTimer: ReturnType<typeof setTimeout> | null = null;

/** 会话内当日计数（跨 composable 调用共享）。 */
let dayKey = "";
let budgetUsed = 0;
let lastFireAt: number | null = null;
/** 当日已触发过的锚点集合（锚点去重，裁决 #3）。 */
let firedAnchors = new Set<number>();

/** 跨天重置（本地日期键）。 */
function rollDayIfNeeded(now: number): void {
  const key = new Date(now).toDateString();
  if (key !== dayKey) {
    dayKey = key;
    budgetUsed = 0;
    lastFireAt = null;
    firedAnchors = new Set();
  }
}

/** 勿扰时段解析（"HH:MM"；解析失败的段忽略）。 */
function inDndRange(now: number): boolean {
  const dnd = settingsStore.snapshot?.dnd;
  if (!dnd?.enabled || !dnd.ranges?.length) return false;
  const d = new Date(now);
  const minutes = d.getHours() * 60 + d.getMinutes();
  const toMin = (s: string): number | null => {
    const m = /^(\d{1,2}):(\d{2})$/.exec(s.trim());
    return m ? Number(m[1]) * 60 + Number(m[2]) : null;
  };
  return dnd.ranges.some((r) => {
    const s = toMin(r.start);
    const e = toMin(r.end);
    return s != null && e != null && minutes >= s && minutes <= e;
  });
}

/** 命中的锚点序号（无则 -1；用于锚点当日去重）。 */
function hitAnchorIndex(now: number): number {
  const d = new Date(now);
  const minutes = d.getHours() * 60 + d.getMinutes();
  return ANCHOR_WINDOWS.findIndex((a) => minutes >= a.startMin && minutes <= a.endMin);
}

/** petStore.lastInteraction（"YYYY-MM-DD HH:MM:SS" 本地）→ 毫秒；无效 → null。 */
function lastInteractionMs(): number | null {
  const raw = petStore.lastInteraction;
  if (!raw) return null;
  const ms = Date.parse(raw.replace(" ", "T"));
  return Number.isFinite(ms) ? ms : null;
}

/** 本 tick 仲裁（now/rng 注入纯函数引擎；锚点当日去重在此组装）。 */
function tick(): void {
  const now = Date.now();
  rollDayIfNeeded(now);
  // D4.4 边缘吸附休眠：主动引擎 skip（正交勿扰，内存态）
  if (settingsStore.sleepSnap) return;

  const anchorIndex = hitAnchorIndex(now);
  const ctx: ProactiveCtx = {
    now,
    tier: settingsStore.frequency,
    dndEnabled: settingsStore.snapshot?.dnd.enabled ?? false,
    inDndRange: inDndRange(now),
    foreground: probeStore.foreground,
    budgetUsed,
    lastFireAt,
    anchorBoostSuppressed: anchorIndex >= 0 && firedAnchors.has(anchorIndex),
  };
  if (!shouldFire(ctx, Math.random)) return;

  lastFireAt = now;
  budgetUsed += 1;
  if (anchorIndex >= 0) firedAnchors.add(anchorIndex);

  const tsundere = isTsundere(lastInteractionMs(), now);
  void proactiveGenerate(tsundere).catch((e: unknown) => {
    console.warn("[proactive] 生成请求失败（下个 tick 再抽签）", e);
  });
}

/** 气泡打字机（本地逐字；新气泡清旧定时器，卸载全清）。 */
function showBubble(p: ProactiveShowPayload): void {
  stopTypewriter();
  setExpression(p.mood);
  proactiveBubble.text = "";
  proactiveBubble.isStreaming = true;
  const full = p.text;
  let i = 0;
  typeTimer = setInterval((): void => {
    if (i >= full.length) {
      stopTypewriter();
      proactiveBubble.isStreaming = false;
      ttlTimer = setTimeout(
        (): void => {
          proactiveBubble.text = "";
          ttlTimer = null;
        },
        bubbleTtlMs(full, p.mood),
      );
      return;
    }
    proactiveBubble.text += full.slice(i, i + TYPEWRITER_CHUNK);
    i += TYPEWRITER_CHUNK;
  }, TYPEWRITER_STEP_MS);
}

function stopTypewriter(): void {
  if (typeTimer) {
    clearInterval(typeTimer);
    typeTimer = null;
  }
  if (ttlTimer) {
    clearTimeout(ttlTimer);
    ttlTimer = null;
  }
}

export function useProactive(): void {
  onMounted((): void => {
    void (async (): Promise<void> => {
      try {
        unlisteners.push(await onProactiveShow(showBubble));
      } catch (e) {
        console.warn("[proactive] 事件订阅失败", e);
      }
    })();
    // 拉一次真实 lastInteraction（傲娇判定数据源；含 pet_get_state 懒结算副作用）
    void getState()
      .then(applyPetState)
      .catch((e: unknown) => {
        console.warn("[proactive] 状态拉取失败（傲娇判定退化）", e);
      });
    tickTimer = setInterval(tick, PROACTIVE_TICK_MS);
  });

  onUnmounted((): void => {
    if (tickTimer) {
      clearInterval(tickTimer);
      tickTimer = null;
    }
    stopTypewriter();
    for (const un of unlisteners.splice(0)) un();
  });
}

/** 供测试/调试：导出最小间隔常量语义（防误删）。 */
export const __MIN_FIRE_INTERVAL_MS = MIN_FIRE_INTERVAL_MS;

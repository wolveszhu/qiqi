/**
 * 广告关闭状态（单例 composable，同 useUpdate/useWebSearchPreference 模式）。
 *
 * 数据流：关闭日期由 Rust 以本地时区写入 `app_settings`（唯一持久化源），
 * 本模块读取映射并按「本地今日」判定可见性——昨日及更早条目自然失效，次日自动恢复。
 * 广告内容为本地常量（`src/config/ads.ts`），本模块只管状态与动作，不渲染 UI。
 */

import { ref, type Ref } from "vue";

import { getAdDismissals, markAdDismissed } from "../api/settings";
import { normalizeError } from "../api/client";
import { pushToast } from "./useToast";

/** 关闭映射 `{素材ID: YYYY-MM-DD}`（加载失败回落空映射 = 全部可见）。 */
const dismissals = ref<Record<string, string>>({});
let listenerInitialized = false;

/**
 * 本地今日 `YYYY-MM-DD`（避开 `toISOString()` 的 UTC 偏移坑）。
 * @param d - 待格式化的日期（测试可注入）
 * @returns 本地时区的日期串
 */
export function localToday(d: Date = new Date()): string {
  const pad = (n: number): string => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/**
 * 是否今日已关闭（导出以便单测；昨日及更早自然失效）。
 * @param date - 记录的关闭日期
 * @param today - 对比基准（默认本地今日，测试可注入）
 */
export function isDismissedToday(
  date: string | undefined,
  today: string = localToday(),
): boolean {
  return date === today;
}

/** 加载关闭映射（幂等；失败仅告警并回落全部可见，不阻断设置窗）。 */
export async function initAds(): Promise<void> {
  if (listenerInitialized) return;
  listenerInitialized = true;
  try {
    dismissals.value = await getAdDismissals();
  } catch (e: unknown) {
    console.warn("[ad] 广告关闭状态读取失败，回落全部可见", e);
  }
}

/** 关闭某广告（**写库成功后才隐藏**；失败 toast 且保持展示，不静默假装已关闭）。 */
export async function dismissAd(adId: string): Promise<void> {
  try {
    await markAdDismissed(adId);
    dismissals.value = { ...dismissals.value, [adId]: localToday() };
  } catch (e: unknown) {
    pushToast(normalizeError(e).message, "error");
  }
}

/** 消费广告状态（组件/容器层）。 */
export interface UseAds {
  /** 关闭映射（只读消费；写入走 `dismissAd`）。 */
  dismissals: Ref<Record<string, string>>;
  /** 某广告位是否可见（读响应式状态，模板调用可自动重渲染）。 */
  isVisible: (adId: string) => boolean;
  /** 关闭某广告（今日不再显示）。 */
  dismissAd: (adId: string) => Promise<void>;
}

export function useAds(): UseAds {
  const isVisible = (adId: string): boolean => !isDismissedToday(dismissals.value[adId]);
  return { dismissals, isVisible, dismissAd };
}

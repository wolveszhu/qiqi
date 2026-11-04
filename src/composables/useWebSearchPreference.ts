/**
 * 联网搜索「全局默认」共享状态（单例，同 useToast/useUpdate 模式）。
 *
 * 数据流（单一真相源）：
 * - **唯一持久化开关** = `search_settings.enabled`（LlmPanel「启用联网搜索」读写，
 *   后端 `is_ready()` 据此裁决是否真正发起搜索）。
 * - ChatPanel 的对话开关是**会话级内存态**：初始/新对话/全局变化时取本全局默认；
 *   对话框内的切换只作用于当前对话，**不回写、不落库**（用户决策：全局开 →
 *   对话框保持开；对话框开 → 仅当前对话）。
 */

import { ref, type Ref } from "vue";

import { getSearchSettings } from "../api/settings";

/** 全局默认开关（`search_settings.enabled` 的前端镜像；读取失败回落 false）。 */
const globalEnabled = ref(false);

/**
 * 从后端加载全局默认（每次实拉保证跨面板新鲜度；失败仅告警并回落 false，不抛错）。
 * @returns void
 */
export async function loadGlobalDefault(): Promise<void> {
  try {
    const s = await getSearchSettings();
    globalEnabled.value = s.enabled;
  } catch (e: unknown) {
    console.warn("[web-search] 全局联网配置读取失败，回落关闭", e);
  }
}

/**
 * 更新全局默认（LlmPanel 读取/保存成功后调用；对话框 watch 此值跟随）。
 * @param v - 新的全局开关值
 * @returns void
 */
export function setGlobalDefault(v: boolean): void {
  globalEnabled.value = v;
}

/** 消费全局默认（只读；对话框据此初始化/重置会话级开关）。 */
export interface UseWebSearchPreference {
  /** 全局默认开关（只读消费；写入走 `setGlobalDefault`）。 */
  globalEnabled: Ref<boolean>;
}

export function useWebSearchPreference(): UseWebSearchPreference {
  return { globalEnabled };
}

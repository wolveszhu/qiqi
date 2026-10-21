/**
 * 好感度事件消费（骨架占位）：affection:changed → petStore + 表情联动。
 *
 * TODO(C5/D2): 监听 affection:changed（delta/newLevel/mood）→ applyPetState + setExpression；
 * 纪律：窗口 A 只取 mood 驱动表情，数值仅进窗口 B 看板（禁播报红线）。
 */

import { onUnmounted } from "vue";

import type { UnlistenFn } from "@tauri-apps/api/event";

const unlisteners: UnlistenFn[] = [];

export function useAffection(): void {
  // TODO(C5): listen("affection:changed", ...)
  onUnmounted((): void => {
    for (const un of unlisteners.splice(0)) un();
  });
}

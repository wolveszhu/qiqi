/**
 * 系统探针事件订阅（骨架占位）：C2 事件 → 渲染器/微随机引擎。
 *
 * TODO(C2): listen("probe:cursor:move") → lookAt；listen("window:chat:toggle") → ChatInput；
 * TODO(C6): listen("probe:idle:changed" | "probe:foreground:changed") → 微随机快照。
 * 纪律：每个 listen 必须持有 unlisten 并在 onUnmounted 调用（E1a 事件矩阵核对项）。
 */

import { onUnmounted } from "vue";

import type { UnlistenFn } from "@tauri-apps/api/event";

const unlisteners: UnlistenFn[] = [];

export function useSystemProbe(): void {
  // TODO(C2/C6): const un = await listen("probe:cursor:move", (e) => ...); unlisteners.push(un);
  onUnmounted((): void => {
    for (const un of unlisteners.splice(0)) un();
  });
}

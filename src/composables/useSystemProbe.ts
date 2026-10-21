/**
 * 系统探针事件订阅（C2 实装）：`probe:cursor:move` → 渲染器视线跟随。
 *
 * 纪律（E1a 事件矩阵核对项）：每个 `listen` 必须持有 unlisten 并在
 * `onUnmounted` 调用；错误只 warn 不吞（探针故障不影响桌宠运行）。
 *
 * TODO(C6): 订阅 `probe:idle:changed` / `probe:foreground:changed` → 微随机快照。
 * TODO(D1): 订阅 `window:chat:toggle` → 输入框显隐（api/probe.ts 已备）。
 */

import { onUnmounted } from "vue";

import { onCursorMove } from "../api/probe";
import type { UnlistenFn } from "@tauri-apps/api/event";

const unlisteners: UnlistenFn[] = [];

/** 订阅 cursor 事件并把坐标转发给调用方（渲染器 `lookAt`）。 */
export function useSystemProbe(onMove: (x: number, y: number) => void): void {
  void (async (): Promise<void> => {
    try {
      const un = await onCursorMove(onMove);
      unlisteners.push(un);
    } catch (e) {
      // 探针订阅失败（如窗口未就绪）：告警不吞，桌宠其余功能继续
      console.warn("[probe] cursor 订阅失败", e);
    }
  })();

  onUnmounted((): void => {
    for (const un of unlisteners.splice(0)) un();
  });
}

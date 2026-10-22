/**
 * 系统探针事件订阅（C2/C6）：cursor → lookAt；idle/foreground → probeStore。
 *
 * 纪律（E1a 事件矩阵核对项）：每个 `listen` 必须持有 unlisten 并在
 * `onUnmounted` 调用；错误只 warn 不吞（探针故障不影响桌宠运行）。
 *
 * idle 翻转同时驱动动作回调（PetWindow：sleep/stretch），保持
 * "listen 只在 api/ 与本文件"的纪律。
 */

import { onUnmounted } from "vue";

import { onCursorMove, onForegroundChanged, onIdleChanged } from "../api/probe";
import { applyForeground, applyIdle } from "../state/probeStore";
import type { UnlistenFn } from "@tauri-apps/api/event";

const unlisteners: UnlistenFn[] = [];

export interface UseSystemProbeOptions {
  /** cursor 坐标回调（渲染器 lookAt）。 */
  onMove?: (x: number, y: number) => void;
  /** idle 翻转回调（true=入睡 / false=回归）。 */
  onIdleChange?: (idle: boolean) => void;
}

/** 订阅探针事件：cursor 转发 + idle/foreground 写快照。 */
export function useSystemProbe(options: UseSystemProbeOptions = {}): void {
  void (async (): Promise<void> => {
    try {
      if (options.onMove) {
        const onMove = options.onMove;
        unlisteners.push(await onCursorMove(onMove));
      }
      unlisteners.push(
        await onIdleChanged((p) => {
          applyIdle(p.state === "idle", p.idleMs);
          options.onIdleChange?.(p.state === "idle");
        }),
      );
      unlisteners.push(
        await onForegroundChanged((p) => {
          applyForeground(p.process);
        }),
      );
    } catch (e) {
      // 探针订阅失败（如窗口未就绪）：告警不吞，桌宠其余功能继续
      console.warn("[probe] 订阅失败", e);
    }
  })();

  onUnmounted((): void => {
    for (const un of unlisteners.splice(0)) un();
  });
}

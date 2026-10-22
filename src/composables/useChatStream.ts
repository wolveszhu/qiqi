/**
 * 聊天流消费（C4.7 实装，替换占位）：
 *
 * - `chat:stream:delta` → **按帧节流**合并后写入 chatStore（父计划风险表对策：
 *   Rust 逐 delta emit，前端按 rAF 合帧渲染，防打字机卡顿）；
 * - `chat:stream:done` → 结束流 + 表情 happy（C1 契约 `setExpression`）；
 * - `chat:stream:error` → 结束流（错误已在 Rust 侧规整；abort/网络均可达）。
 *
 * 纪律：listen 全部持有 unlisten 并在 onUnmounted 清理（E1a 事件矩阵）；
 * 流式文本只写入 chatStore 的纯字符串，不进深层响应式结构。
 */

import { onUnmounted } from "vue";

import { listen } from "@tauri-apps/api/event";

import { abort, sendStream } from "../api/chat";
import { appendDelta, clearBubble, finishStream, startSession } from "../state/chatStore";
import { setExpression } from "./usePetRenderer";
import type { FilePayload } from "../api/types";
import type { UnlistenFn } from "@tauri-apps/api/event";

/** 帧节流缓冲（delta 先入缓冲，rAF 时一次性 flush）。 */
let pendingText = "";
let rafId: number | null = null;
/** 当前会话（事件过滤：只消费本会话的流，防串扰）。 */
let activeSessionId = "";
const unlisteners: UnlistenFn[] = [];

/** 按帧 flush 缓冲。 */
function flushPending(): void {
  rafId = null;
  if (!pendingText) return;
  appendDelta(pendingText);
  pendingText = "";
}

function scheduleFlush(): void {
  if (rafId !== null) return;
  rafId = requestAnimationFrame(flushPending);
}

/** useChatStream 暴露的 API（PetWindow / ChatInput 调用）。 */
export interface UseChatStreamApi {
  /** 发送一条消息（生成 sessionId 并启动流式接收；attachments 可选，D1.5 多模态）。 */
  send: (text: string, attachments?: FilePayload[]) => void;
  /** 中止当前流。 */
  abortCurrent: () => void;
}

export function useChatStream(): UseChatStreamApi {
  void (async (): Promise<void> => {
    try {
      unlisteners.push(
        await listen<{ sessionId: string; delta: string }>(
          "chat:stream:delta",
          (event) => {
            if (event.payload.sessionId !== activeSessionId) return;
            pendingText += event.payload.delta;
            scheduleFlush();
          },
        ),
        await listen<{ sessionId: string }>("chat:stream:done", (event) => {
          if (event.payload.sessionId !== activeSessionId) return;
          flushPending();
          finishStream();
          void setExpression("happy"); // done 后短暂开心（C1 契约；E3 打磨期可加复位计时）
        }),
        await listen<{ sessionId: string; kind: string; message: string }>(
          "chat:stream:error",
          (event) => {
            if (event.payload.sessionId !== activeSessionId) return;
            flushPending();
            finishStream();
            // abort 是用户主动行为，不打扰；其余错误在气泡尾部标注（不弹窗）
            if (event.payload.kind !== "aborted") {
              appendDelta("……（刚才好像断线了）");
              console.warn("[chat] 流错误", event.payload);
            }
          },
        ),
      );
    } catch (e) {
      console.warn("[chat] 事件订阅失败", e);
    }
  })();

  onUnmounted((): void => {
    if (rafId !== null) cancelAnimationFrame(rafId);
    for (const un of unlisteners.splice(0)) un();
  });

  return {
    send: (text: string, attachments: FilePayload[] = []): void => {
      const content = text.trim();
      if (!content && attachments.length === 0) return;
      const sessionId = crypto.randomUUID();
      activeSessionId = sessionId;
      startSession(sessionId);
      void sendStream(sessionId, content, attachments).catch((e: unknown) => {
        // 命令本身失败（如 DB 故障）：结束流 + 提示，不崩溃
        finishStream();
        appendDelta("……（好像没连上我自己的脑子）");
        console.warn("[chat] send 失败", e);
      });
    },
    abortCurrent: (): void => {
      if (activeSessionId) void abort(activeSessionId).catch(() => {});
      clearBubble();
    },
  };
}

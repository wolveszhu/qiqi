/**
 * 聊天流消费（C4.7；**窗口 B「对话」Tab** 使用）。
 *
 * - `chat:stream:delta` → **按帧节流**合并后写入 `chatStore.streamingText`
 *   （Rust 逐 delta emit，前端 rAF 合帧渲染，防打字机卡顿）；
 * - `chat:stream:done` → 流文本并入消息区 + emit `chat:done`（窗口 A 监听后做表情联动，
 *   因为 B 无渲染器句柄，不能直调 `setExpression`）；
 * - `chat:stream:error` → 结束流（错误已由 Rust 规整；abort/网络均可达）。
 *
 * 多轮：**复用当前会话 id**（`chatStore.currentId`）——Rust 侧按 session_id 取历史与会话摘要，
 * 故续聊上下文由后端自动带上；无当前会话时才生成新 id。
 *
 * 纪律：listen 全部持有 unlisten 并在 onUnmounted 清理（E1a 事件矩阵）。
 */

import { onUnmounted } from "vue";

import { emit, listen } from "@tauri-apps/api/event";

import { abort, sendStream } from "../api/chat";
import {
  appendDelta,
  clearStream,
  finishStream,
  pushUserMessage,
  startStream,
} from "../state/chatStore";
import { chatStore } from "../state/chatStore";
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

/** useChatStream 暴露的 API（ChatPanel 调用）。 */
export interface UseChatStreamApi {
  /** 发送一条消息（续聊当前会话；无会话则新建；attachments 可选，D1.5 多模态）。 */
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
          // 表情联动在窗口 A：B 无渲染器，只能发事件（A 侧 onChatDone → setExpression happy）
          void emit("chat:done", { sessionId: event.payload.sessionId });
        }),
        await listen<{ sessionId: string; kind: string; message: string }>(
          "chat:stream:error",
          (event) => {
            if (event.payload.sessionId !== activeSessionId) return;
            flushPending();
            // abort 是用户主动行为，不打扰；其余错误在回复尾部标注（不弹窗）
            if (event.payload.kind !== "aborted") {
              appendDelta("……（刚才好像断线了）");
              console.warn("[chat] 流错误", event.payload);
            }
            finishStream();
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
      // 续聊：有 currentId 就复用（多轮上下文由 Rust 按 session_id 组装）
      const sessionId = chatStore.currentId || crypto.randomUUID();
      activeSessionId = sessionId;
      startStream(sessionId);
      pushUserMessage(content || "（附件）");
      void sendStream(sessionId, content, attachments).catch((e: unknown) => {
        // 命令本身失败（如 DB 故障）：结束流 + 提示，不崩溃
        appendDelta("……（好像没连上我自己的脑子）");
        finishStream();
        console.warn("[chat] send 失败", e);
      });
    },
    abortCurrent: (): void => {
      if (activeSessionId) void abort(activeSessionId).catch(() => {});
      clearStream();
    },
  };
}

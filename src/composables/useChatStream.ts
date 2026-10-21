/**
 * 对话流消费（骨架占位）：chat:stream:* 事件 → chatStore（帧节流写入）。
 *
 * TODO(C4): send(sessionId, text) → chat_send_stream；delta 按帧合并 appendDelta；
 * done → finishStream；error → 气泡友好提示（禁弹系统框）。
 * TODO(D1): attachments 透传。
 */

import { onUnmounted } from "vue";

import type { UnlistenFn } from "@tauri-apps/api/event";

const unlisteners: UnlistenFn[] = [];

/** useChatStream 暴露的 API（供 PetWindow/ChatInput 调用）。 */
export interface UseChatStreamApi {
  send: (text: string, attachments?: unknown[]) => void;
  abort: () => void;
}

export function useChatStream(): UseChatStreamApi {
  // TODO(C4): listen("chat:stream:delta" | "chat:stream:done" | "chat:stream:error")
  onUnmounted((): void => {
    for (const un of unlisteners.splice(0)) un();
  });

  return {
    /** TODO(C4): 生成 sessionId + 调 chat_send_stream */
    send: (_text: string, _attachments?: unknown[]) => {
      // 占位
    },
    /** TODO(C4): chat_abort */
    abort: () => {
      // 占位
    },
  };
}

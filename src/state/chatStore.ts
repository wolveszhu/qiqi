/**
 * 聊天气泡状态（规范 §3：窗口 A 单气泡覆盖式，勘误#12）。
 *
 * TODO(C4): useChatStream 以 chat:stream:delta 事件节流写入；
 * 气泡文本是纯字符串 ref 语义，不进响应式深结构（性能约束）。
 */

import { reactive } from "vue";

import type { ChatBubbleState } from "../api/types";

const state = reactive<ChatBubbleState>({
  sessionId: "",
  text: "",
  isStreaming: false,
});

/** 开始新会话（send 时生成 crypto.randomUUID()）。 */
export function startSession(sessionId: string): void {
  state.sessionId = sessionId;
  state.text = "";
  state.isStreaming = true;
}

/** 流式增量（前端按帧合并后调用，防打字机卡顿）。 */
export function appendDelta(delta: string): void {
  state.text += delta;
}

/** 流结束（done/error 均置位）。 */
export function finishStream(): void {
  state.isStreaming = false;
}

export function clearBubble(): void {
  state.text = "";
  state.isStreaming = false;
}

export const chatStore = state;

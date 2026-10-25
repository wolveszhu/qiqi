/**
 * 多会话对话状态（窗口 B「对话」Tab；窗口 A 不再消费流式文本——只显示主动话术气泡）。
 *
 * 纪律：
 * - 会话列表与消息列表进响应式（UI 列表需要）；**流式文本单独用纯字符串字段**，
 *   打字机每 token 只改这一个字段，不触发消息数组 diff（父计划风险表对策）。
 * - `currentId` 为**内存态**：重启即新会话（用户决策），不落库；历史仍可从会话列表回看。
 */

import { reactive } from "vue";

import type { ChatMessageDto, ChatSessionDto, ChatSource } from "../api/types";

/** 消息视图模型（消息区渲染用）。 */
export interface ChatMessageVm {
  /** 列表 key：DB 消息为 `db-<id>`，乐观插入为 `local-<seq>`（避免与 DB 自增 id 冲突）。 */
  key: string;
  /** 0=user / 1=assistant（与 Rust `messages.role` 一致）。 */
  role: number;
  content: string;
  /** 联网搜索引用来源（仅 assistant 消息可能携带；气泡底部渲染）。 */
  sources?: ChatSource[];
}

export interface ChatState {
  /** 会话列表（按最后活跃倒序）。 */
  sessions: ChatSessionDto[];
  /** 当前会话 id；空串 = 尚未创建（首次发送时生成）。 */
  currentId: string;
  /** 当前会话的消息区内容（时间正序）。 */
  messages: ChatMessageVm[];
  /** 当前流的增量文本（尚未并入消息区）。 */
  streamingText: string;
  isStreaming: boolean;
  /** 当前流的引用来源（chat:stream:sources 下发；finishStream 时并入 assistant 消息）。 */
  streamingSources: ChatSource[];
}

const state = reactive<ChatState>({
  sessions: [],
  currentId: "",
  messages: [],
  streamingText: "",
  isStreaming: false,
  streamingSources: [],
});

/** 乐观插入的本地序号（与 DB id 空间隔离）。 */
let localSeq = 0;

/** 覆盖会话列表（后端拉取后调用）。 */
export function setSessions(list: ChatSessionDto[]): void {
  state.sessions = list;
}

/** 从列表中移除会话（删除成功后本地同步，避免再拉一次列表）。 */
export function removeSession(sessionId: string): void {
  state.sessions = state.sessions.filter((s) => s.sessionId !== sessionId);
}

/** 切换会话：清空消息区与流式缓冲（消息由调用方随后加载）。 */
export function switchSession(sessionId: string): void {
  state.currentId = sessionId;
  state.messages = [];
  state.streamingText = "";
  state.isStreaming = false;
}

/** 新建会话：置空 currentId（下次发送时生成 id），消息区立即变空白。 */
export function startNewSession(): void {
  switchSession("");
}

/** 用后端消息覆盖消息区（时间正序）。 */
export function setMessages(list: ChatMessageDto[]): void {
  state.messages = list.map((m) => ({
    key: `db-${m.id}`,
    role: m.role,
    content: m.content,
  }));
}

/** 乐观插入一条用户消息（Rust 已在 send 时落库，此处只补 UI）。 */
export function pushUserMessage(text: string): void {
  localSeq += 1;
  state.messages.push({ key: `local-${localSeq}`, role: 0, content: text });
}

/** 开始一轮流（会话 id 由调用方决定：续聊用现有 id，新会话用新建 id）。 */
export function startStream(sessionId: string): void {
  state.currentId = sessionId;
  state.streamingText = "";
  state.streamingSources = [];
  state.isStreaming = true;
}

/** 流式增量（已按帧合帧后调用，防打字机卡顿）。 */
export function appendDelta(delta: string): void {
  state.streamingText += delta;
}

/** 设置当前流的引用来源（chat:stream:sources 事件回调）。 */
export function setStreamSources(sources: ChatSource[]): void {
  state.streamingSources = sources;
}

/** 流结束：把流文本并入消息区作为 assistant 消息（空文本则丢弃，如 abort）。 */
export function finishStream(): void {
  state.isStreaming = false;
  const text = state.streamingText;
  const sources = state.streamingSources;
  state.streamingText = "";
  state.streamingSources = [];
  if (!text) return;
  localSeq += 1;
  state.messages.push({
    key: `local-${localSeq}`,
    role: 1,
    content: text,
    sources: sources.length ? [...sources] : undefined,
  });
}

/** 清空当前流缓冲（中止场景：部分文本不作为完整回复）。 */
export function clearStream(): void {
  state.streamingText = "";
  state.isStreaming = false;
}

export const chatStore = state;

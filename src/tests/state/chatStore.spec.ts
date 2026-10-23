/**
 * 多会话对话状态（chatStore）行为契约（F2）。
 *
 * 覆盖点：会话列表同步、消息映射、流式文本并入消息区、空流丢弃、切换/新建会话清空、
 * 删除会话本地移除——均为纯状态逻辑，不依赖 Tauri/DOM。
 */

import { beforeEach, describe, expect, it } from "vitest";

import {
  appendDelta,
  clearStream,
  finishStream,
  pushUserMessage,
  removeSession,
  setMessages,
  setSessions,
  startNewSession,
  startStream,
  switchSession,
} from "../../state/chatStore";
import { chatStore } from "../../state/chatStore";
import type { ChatMessageDto, ChatSessionDto } from "../../api/types";

function session(id: string, preview: string): ChatSessionDto {
  return { sessionId: id, preview, lastAt: "2026-10-24 10:00:00", messageCount: 2 };
}

function msg(id: number, role: number, content: string): ChatMessageDto {
  return { id, role, content };
}

describe("chatStore", () => {
  beforeEach(() => {
    setSessions([]);
    switchSession("");
    clearStream();
  });

  it("setMessages 映射为 db-<id> key 且保持顺序", () => {
    setMessages([msg(3, 0, "第一句"), msg(7, 1, "第二句")]);
    expect(chatStore.messages.map((m) => m.key)).toEqual(["db-3", "db-7"]);
    expect(chatStore.messages.map((m) => m.content)).toEqual(["第一句", "第二句"]);
    expect(chatStore.messages[1].role).toBe(1);
  });

  it("流结束后 streamingText 并入消息区作为 assistant 消息", () => {
    startStream("s1");
    expect(chatStore.currentId).toBe("s1");
    appendDelta("你");
    appendDelta("好呀");
    expect(chatStore.isStreaming).toBe(true);

    finishStream();

    expect(chatStore.isStreaming).toBe(false);
    expect(chatStore.streamingText).toBe("");
    const last = chatStore.messages[chatStore.messages.length - 1];
    expect(last.role).toBe(1);
    expect(last.content).toBe("你好呀");
    expect(last.key.startsWith("local-")).toBe(true);
  });

  it("空流结束不产生 assistant 消息（abort 场景）", () => {
    startStream("s1");
    finishStream();
    expect(chatStore.messages).toHaveLength(0);
  });

  it("pushUserMessage 追加 user 消息且不与 db key 冲突", () => {
    setMessages([msg(1, 0, "旧消息")]);
    pushUserMessage("新提问");
    expect(chatStore.messages).toHaveLength(2);
    expect(chatStore.messages[1].role).toBe(0);
    expect(chatStore.messages[1].key.startsWith("local-")).toBe(true);
  });

  it("switchSession 清空消息与流缓冲并切换 currentId", () => {
    startStream("s1");
    appendDelta("半截话");
    switchSession("s2");
    expect(chatStore.currentId).toBe("s2");
    expect(chatStore.messages).toHaveLength(0);
    expect(chatStore.streamingText).toBe("");
    expect(chatStore.isStreaming).toBe(false);
  });

  it("startNewSession 置空 currentId（下次发送才生成 id）", () => {
    switchSession("s1");
    startNewSession();
    expect(chatStore.currentId).toBe("");
  });

  it("removeSession 只移除目标会话", () => {
    setSessions([session("s1", "甲"), session("s2", "乙")]);
    removeSession("s1");
    expect(chatStore.sessions.map((s) => s.sessionId)).toEqual(["s2"]);
  });

  it("clearStream 丢弃未完成的流文本", () => {
    startStream("s1");
    appendDelta("半截");
    clearStream();
    expect(chatStore.streamingText).toBe("");
    expect(chatStore.isStreaming).toBe(false);
    expect(chatStore.messages).toHaveLength(0);
  });
});

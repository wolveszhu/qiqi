import { describe, it, expect, beforeEach } from "vitest";
import {
  chatStore,
  finishStream,
  setStreamSources,
  startStream,
} from "../../state/chatStore";

/**
 * 联网搜索来源挂载：sources 事件 → streamingSources → finishStream 并入 assistant 消息。
 * 注：chatStore 为模块级单例，测试间用 startStream 重置缓冲。
 */
describe("chatStore 联网来源挂载", () => {
  beforeEach(() => {
    startStream("sess-x");
  });

  it("finishStream 把 streamingSources 挂到 assistant 消息", () => {
    setStreamSources([
      { title: "实时新闻", url: "https://news.test/a" },
      { title: "百科", url: "https://wiki.test/b" },
    ]);
    chatStore.streamingText = "这是联网回答";
    finishStream();

    const last = chatStore.messages[chatStore.messages.length - 1];
    expect(last.role).toBe(1);
    expect(last.sources).toBeDefined();
    expect(last.sources?.length).toBe(2);
    expect(last.sources?.[0].url).toBe("https://news.test/a");
    // 并入后清空缓冲
    expect(chatStore.streamingSources).toEqual([]);
  });

  it("无来源时 assistant 消息不含 sources 字段", () => {
    chatStore.streamingText = "普通回答";
    finishStream();
    const last = chatStore.messages[chatStore.messages.length - 1];
    expect(last.sources).toBeUndefined();
  });

  it("空流文本不并入消息（如 abort）", () => {
    const before = chatStore.messages.length;
    finishStream();
    expect(chatStore.messages.length).toBe(before);
  });
});

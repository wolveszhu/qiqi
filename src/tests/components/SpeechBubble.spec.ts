// @vitest-environment happy-dom
/**
 * SpeechBubble 组件行为契约（F2.3 / D1）：流式文本渲染、打字光标、
 * 空文本不渲染、**窗口 A 红线：只渲染传入文本，零数值注入**。
 *
 * 布局（翻转/防截断）已由 utils/bubbleLayout 纯函数覆盖（tests/utils/bubbleLayout.spec.ts），
 * 此处只断言渲染与契约，不断言真实像素（像素归 F3 手工）。
 */

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import SpeechBubble from "../../components/pet/SpeechBubble.vue";

describe("SpeechBubble", () => {
  it("可见且有文本 → 渲染文本 + role=status", () => {
    const w = mount(SpeechBubble, { props: { text: "在吗", visible: true } });
    const bubble = w.get(".bubble");
    expect(bubble.attributes("role")).toBe("status");
    expect(bubble.attributes("aria-live")).toBe("polite");
    expect(w.get(".bubble-text").text()).toBe("在吗");
  });

  it("流式 → 显示打字光标 caret", async () => {
    const w = mount(SpeechBubble, { props: { text: "打字中", streaming: true } });
    expect(w.find(".caret").exists()).toBe(true);
    await w.setProps({ streaming: false });
    expect(w.find(".caret").exists()).toBe(false);
  });

  it("空文本 → 不渲染气泡", () => {
    const w = mount(SpeechBubble, { props: { text: "", visible: true } });
    expect(w.find(".bubble").exists()).toBe(false);
  });

  it("visible=false → 不渲染气泡", () => {
    const w = mount(SpeechBubble, { props: { text: "隐藏", visible: false } });
    expect(w.find(".bubble").exists()).toBe(false);
  });

  it("长文本不截断：maxWidth 生效且全文渲染", () => {
    const long = "喵".repeat(200);
    const w = mount(SpeechBubble, { props: { text: long } });
    const style = w.get(".bubble").attributes("style") ?? "";
    expect(style).toMatch(/max-width:\s*\d+px/);
    // 全文进入 DOM（截断由 CSS max-width + break-word 处理，文本不丢）
    expect(w.get(".bubble-text").text()).toBe(long);
  });

  it("窗口 A 红线：渲染输出恰为传入文本，无任何数值/好感度注入", () => {
    const w = mount(SpeechBubble, { props: { text: "今天也要加油喵" } });
    const text = w.get(".bubble").text().replace(/\s+/g, "");
    // 气泡可见文本必须与输入一致——拦截"顺手把 affection 数值拼进气泡"的回归
    expect(text).toBe("今天也要加油喵");
    expect(text).not.toMatch(/\d/);
  });

  it("布局 class 由 resolveBubbleLayout 决定（side 来自纯函数）", () => {
    const w = mount(SpeechBubble, { props: { text: "x" } });
    const cls = w.get(".bubble").classes();
    expect(cls.some((c) => c === "left" || c === "right")).toBe(true);
  });
});

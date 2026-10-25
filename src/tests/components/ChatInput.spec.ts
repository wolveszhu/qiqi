// @vitest-environment happy-dom
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import ChatInput from "../../components/pet/ChatInput.vue";

describe("ChatInput 联网搜索开关", () => {
  it("默认 webSearch=关闭（开关无 is-on 类）", () => {
    const w = mount(ChatInput, { props: { webSearch: false } });
    expect(w.find(".composer__web.is-on").exists()).toBe(false);
  });

  it("点击开关 emit update:webSearch 翻转", async () => {
    const w = mount(ChatInput, { props: { webSearch: false } });
    await w.find(".composer__web").trigger("click");
    expect(w.emitted("update:webSearch")?.[0]).toEqual([true]);

    const w2 = mount(ChatInput, { props: { webSearch: true } });
    await w2.find(".composer__web").trigger("click");
    expect(w2.emitted("update:webSearch")?.[0]).toEqual([false]);
  });

  it("输入非空文本发送 → emit send 且清空草稿", async () => {
    const w = mount(ChatInput, { props: { webSearch: false } });
    await w.find("textarea").setValue("你好");
    await w.find(".composer__send").trigger("click");
    expect(w.emitted("send")?.[0]).toEqual(["你好"]);
    expect((w.find("textarea").element as HTMLTextAreaElement).value).toBe("");
  });

  it("空内容不发送", async () => {
    const w = mount(ChatInput, { props: { webSearch: false } });
    await w.find(".composer__send").trigger("click");
    expect(w.emitted("send")).toBeUndefined();
  });
});

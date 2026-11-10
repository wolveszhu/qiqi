// @vitest-environment happy-dom
/**
 * 划词翻译浮窗组件契约：小图标点击才触发、译文展示与复制/重译/关闭 emit、中英双语。
 */

import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { TranslateResult } from "../../api/types";
import i18n, { DEFAULT_LOCALE } from "../../i18n";
import SelectionToolbar from "../../views/translate/SelectionToolbar.vue";
import TranslationPopup from "../../views/translate/TranslationPopup.vue";

const result: TranslateResult = {
  resultId: "r1",
  sourceText: "Hello",
  translatedText: "你好",
  sourceLang: "en",
  targetLang: "zh",
  provider: "baidu",
  serviceName: "百度翻译",
  isBuiltin: true,
  quotaExhausted: false,
};

afterEach((): void => {
  i18n.global.locale.value = DEFAULT_LOCALE;
});

describe("SelectionToolbar", () => {
  it("翻译 / 复制 / 关闭各自触发一次 emit", async () => {
    const wrapper = mount(SelectionToolbar);
    const actions = wrapper.findAll(".toolbar__action");
    await actions[0].trigger("click");
    await actions[1].trigger("click");
    await wrapper.get(".toolbar__close").trigger("click");
    expect(wrapper.emitted("translate")).toHaveLength(1);
    expect(wrapper.emitted("copy")).toHaveLength(1);
    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("loading 时两个动作按钮均禁用", () => {
    const wrapper = mount(SelectionToolbar, { props: { loading: true } });
    const actions = wrapper.findAll(".toolbar__action");
    expect(actions).toHaveLength(2);
    expect(actions[0].attributes("disabled")).toBeDefined();
    expect(actions[1].attributes("disabled")).toBeDefined();
  });

  it("工具条与动作文案随语言切换", () => {
    const zh = mount(SelectionToolbar);
    expect(zh.get('[role="toolbar"]').attributes("aria-label")).toBe("翻译选中文本");
    expect(zh.text()).toContain("翻译");
    expect(zh.text()).toContain("复制");
    i18n.global.locale.value = "en-US";
    const en = mount(SelectionToolbar);
    expect(en.get('[role="toolbar"]').attributes("aria-label")).toBe(
      "Translate selection",
    );
    expect(en.text()).toContain("Translate");
    expect(en.text()).toContain("Copy");
  });
});

describe("TranslationPopup", () => {
  it("渲染原文、译文与内置通道标识", () => {
    const wrapper = mount(TranslationPopup, { props: { result } });
    expect(wrapper.text()).toContain("Hello");
    expect(wrapper.text()).toContain("你好");
    expect(wrapper.text()).toContain("内置");
  });

  it("关闭 / 复制 / 重新翻译各触发一次对应 emit", async () => {
    const wrapper = mount(TranslationPopup, { props: { result } });
    // 模板顺序：头部关闭、底部复制、底部重新翻译
    const buttons = wrapper.findAll("button");
    await buttons[0].trigger("click");
    await buttons[1].trigger("click");
    await buttons[2].trigger("click");
    expect(wrapper.emitted("close")).toHaveLength(1);
    expect(wrapper.emitted("copy")).toHaveLength(1);
    expect(wrapper.emitted("retry")).toHaveLength(1);
  });

  it("busy 时重新翻译按钮禁用", () => {
    const wrapper = mount(TranslationPopup, { props: { result, busy: true } });
    expect(wrapper.findAll("button")[2].attributes("disabled")).toBeDefined();
  });

  it("额度耗尽显示提示", () => {
    const wrapper = mount(TranslationPopup, {
      props: { result: { ...result, quotaExhausted: true } },
    });
    expect(wrapper.text()).toContain("两家内置服务本月用量均已达到上限");
  });

  it("英文模式下文案切换", () => {
    i18n.global.locale.value = "en-US";
    const wrapper = mount(TranslationPopup, { props: { result } });
    expect(wrapper.text()).toContain("Translation");
    expect(wrapper.text()).toContain("Built-in");
  });
});

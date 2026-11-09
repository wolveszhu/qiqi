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
  isBuiltin: true,
  quotaExhausted: false,
};

afterEach((): void => {
  i18n.global.locale.value = DEFAULT_LOCALE;
});

describe("SelectionToolbar", () => {
  it("点击才触发 capture", async () => {
    const wrapper = mount(SelectionToolbar);
    await wrapper.get("button").trigger("click");
    expect(wrapper.emitted("capture")).toHaveLength(1);
  });

  it("loading 时按钮禁用，避免重复取词", () => {
    const wrapper = mount(SelectionToolbar, { props: { loading: true } });
    expect(wrapper.get("button").attributes("disabled")).toBeDefined();
  });

  it("aria-label 随语言切换", () => {
    const zh = mount(SelectionToolbar).get("button").attributes("aria-label");
    i18n.global.locale.value = "en-US";
    const en = mount(SelectionToolbar).get("button").attributes("aria-label");
    expect(zh).toBe("翻译选中文本");
    expect(en).toBe("Translate selection");
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
    expect(wrapper.text()).toContain("内置额度已用尽");
  });

  it("英文模式下文案切换", () => {
    i18n.global.locale.value = "en-US";
    const wrapper = mount(TranslationPopup, { props: { result } });
    expect(wrapper.text()).toContain("Translation");
    expect(wrapper.text()).toContain("Built-in");
  });
});

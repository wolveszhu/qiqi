// @vitest-environment happy-dom
/**
 * AppToggle 组件行为契约（F2.3）：受控 v-model 语义。
 * 只断言 props→渲染/emit 契约，不断言样式/内部结构。
 */

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import AppToggle from "../../components/ui/AppToggle.vue";

describe("AppToggle", () => {
  it("modelValue=false → aria-checked=false 且无 is-on", () => {
    const w = mount(AppToggle, { props: { modelValue: false } });
    const track = w.get('[role="switch"]');
    expect(track.attributes("aria-checked")).toBe("false");
    expect(track.classes()).not.toContain("is-on");
  });

  it("modelValue=true → aria-checked=true 且 is-on", () => {
    const w = mount(AppToggle, { props: { modelValue: true } });
    const track = w.get('[role="switch"]');
    expect(track.attributes("aria-checked")).toBe("true");
    expect(track.classes()).toContain("is-on");
  });

  it("点击 → emit update:modelValue 取反值（受控，不自行改状态）", async () => {
    const w = mount(AppToggle, { props: { modelValue: false } });
    await w.get('[role="switch"]').trigger("click");
    const events = w.emitted("update:modelValue");
    expect(events).toHaveLength(1);
    expect(events?.[0]).toEqual([true]);
  });

  it("label 传入才渲染标签文案", () => {
    expect(
      mount(AppToggle, { props: { modelValue: false } })
        .find(".app-toggle__label")
        .exists(),
    ).toBe(false);
    const w = mount(AppToggle, { props: { modelValue: false, label: "勿扰" } });
    expect(w.get(".app-toggle__label").text()).toBe("勿扰");
  });
});

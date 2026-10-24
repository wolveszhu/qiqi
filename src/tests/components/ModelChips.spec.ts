// @vitest-environment happy-dom
/**
 * ModelChips 组件行为契约（F2）：回车/逗号添加、去重、删除、退格删尾项、建议过滤。
 */

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import ModelChips from "../../components/settings/ModelChips.vue";

function mountChips(modelValue: string[] = [], suggestions: string[] = []) {
  return mount(ModelChips, { props: { modelValue, suggestions } });
}

describe("ModelChips", () => {
  it("回车添加为标签并清空输入", async () => {
    const w = mountChips();
    const input = w.get("input");
    await input.setValue("deepseek-chat");
    await input.trigger("keydown", { key: "Enter" });

    expect(w.emitted("update:modelValue")).toBeTruthy();
    expect(w.emitted("update:modelValue")?.[0]).toEqual([["deepseek-chat"]]);
    expect((input.element as HTMLInputElement).value).toBe("");
  });

  it("逗号（中英文）也可添加", async () => {
    const w = mountChips();
    const input = w.get("input");
    await input.setValue("m1");
    await input.trigger("keydown", { key: "," });
    expect(w.emitted("update:modelValue")?.[0]).toEqual([["m1"]]);

    const w2 = mountChips(["m1"]);
    await w2.get("input").setValue("m2");
    await w2.get("input").trigger("keydown", { key: "，" });
    expect(w2.emitted("update:modelValue")?.[0]).toEqual([["m1", "m2"]]);
  });

  it("空值与重复值不产生新标签", async () => {
    const w = mountChips(["m1"]);
    await w.get("input").setValue("   ");
    await w.get("input").trigger("keydown", { key: "Enter" });
    expect(w.emitted("update:modelValue")).toBeUndefined();

    await w.get("input").setValue("m1");
    await w.get("input").trigger("keydown", { key: "Enter" });
    expect(w.emitted("update:modelValue")).toBeUndefined();
  });

  it("点 ✕ 删除指定标签", async () => {
    const w = mountChips(["m1", "m2"]);
    await w.get('[aria-label="删除模型 m1"]').trigger("click");
    expect(w.emitted("update:modelValue")?.[0]).toEqual([["m2"]]);
  });

  it("空输入按退格删除最后一个标签", async () => {
    const w = mountChips(["m1", "m2"]);
    await w.get("input").trigger("keydown", { key: "Backspace" });
    expect(w.emitted("update:modelValue")?.[0]).toEqual([["m1"]]);
  });

  it("有输入时退格不删标签", async () => {
    const w = mountChips(["m1"]);
    await w.get("input").setValue("abc");
    await w.get("input").trigger("keydown", { key: "Backspace" });
    expect(w.emitted("update:modelValue")).toBeUndefined();
  });

  it("建议项过滤已添加项且点击即添加", async () => {
    const w = mountChips(["a"], ["a", "b", "c", "d", "e"]);
    const suggests = w.findAll(".chips__suggest");
    // 已添加的 a 不再展示，最多 4 个
    expect(suggests).toHaveLength(4);
    expect(suggests.map((s) => s.text())).toEqual(["+ b", "+ c", "+ d", "+ e"]);

    await suggests[0].trigger("click");
    expect(w.emitted("update:modelValue")?.[0]).toEqual([["a", "b"]]);
  });

  it("失焦时把残留输入补添加", async () => {
    const w = mountChips();
    await w.get("input").setValue("m9");
    await w.get("input").trigger("blur");
    expect(w.emitted("update:modelValue")?.[0]).toEqual([["m9"]]);
  });
});

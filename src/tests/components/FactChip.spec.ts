// @vitest-environment happy-dom
/**
 * FactChip 组件行为契约（F2.3 / D3 记忆修正）：行内编辑确认/取消、
 * 删除二次确认；写操作全部上抛（父级落地），零副作用。
 */

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import FactChip from "../../components/memory/FactChip.vue";
import type { FactDto } from "../../api/types";

const FACT: FactDto = { id: 7, category: 0, content: "喜欢冰美式", confidence: 0.85, createdAt: "" };

describe("FactChip", () => {
  it("展示内容 + conf 百分比徽标", () => {
    const w = mount(FactChip, { props: { fact: FACT } });
    expect(w.get(".fact-chip__content").text()).toBe("喜欢冰美式");
    expect(w.get(".fact-chip__conf").text()).toBe("conf 85%");
  });

  it("conf 已为百分数（>1）时不再乘 100", () => {
    const w = mount(FactChip, { props: { fact: { ...FACT, confidence: 85 } } });
    expect(w.get(".fact-chip__conf").text()).toBe("conf 85%");
  });

  it("点编辑 → 行内 input 预填原内容", async () => {
    const w = mount(FactChip, { props: { fact: FACT } });
    await w.get('button[aria-label="编辑"]').trigger("click");
    const input = w.get("input.fact-chip__input");
    expect((input.element as HTMLInputElement).value).toBe("喜欢冰美式");
  });

  it("回车改内容 → emit edit(id, 新内容)", async () => {
    const w = mount(FactChip, { props: { fact: FACT } });
    await w.get('button[aria-label="编辑"]').trigger("click");
    const input = w.get("input.fact-chip__input");
    await input.setValue("喜欢热美式");
    await input.trigger("keyup.enter");
    const events = w.emitted("edit");
    expect(events?.[0]).toEqual([7, "喜欢热美式"]);
    expect(w.find("input.fact-chip__input").exists()).toBe(false);
  });

  it("Esc 取消 → 不 emit edit（零副作用）", async () => {
    const w = mount(FactChip, { props: { fact: FACT } });
    await w.get('button[aria-label="编辑"]').trigger("click");
    const input = w.get("input.fact-chip__input");
    await input.setValue("改坏了");
    await input.trigger("keyup.esc");
    expect(w.emitted("edit")).toBeUndefined();
    // 退出编辑且草稿已恢复（再次进入应仍是原值）
    await w.get('button[aria-label="编辑"]').trigger("click");
    expect((w.get("input.fact-chip__input").element as HTMLInputElement).value).toBe("喜欢冰美式");
  });

  it("内容未变 → 提交不 emit edit", async () => {
    const w = mount(FactChip, { props: { fact: FACT } });
    await w.get('button[aria-label="编辑"]').trigger("click");
    await w.get("input.fact-chip__input").trigger("keyup.enter");
    expect(w.emitted("edit")).toBeUndefined();
  });

  it("删除需二次确认：首次点击转确认态，再次才 emit delete", async () => {
    const w = mount(FactChip, { props: { fact: FACT } });
    const del = w.get('button[aria-label="删除"]');
    expect(del.text()).toBe("✕");
    await del.trigger("click");
    expect(w.emitted("delete")).toBeUndefined();
    expect(w.get('button[aria-label="删除"]').text()).toBe("确认?");
    await w.get('button[aria-label="删除"]').trigger("click");
    expect(w.emitted("delete")?.[0]).toEqual([7]);
  });
});

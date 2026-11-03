// @vitest-environment happy-dom
/**
 * TimeChip 组件行为契约（F2.3 / D2 勿扰时段）：展示 start–end、
 * 可编辑变体行内改时段、删除事件；ui/ 红线——零业务依赖、仅受控事件上抛。
 */

import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { describe, expect, it } from "vitest";

import TimeChip from "../../components/ui/TimeChip.vue";

const RANGE = { start: "09:00", end: "18:00" };

describe("TimeChip", () => {
  it("非编辑态渲染 start – end 文本", () => {
    const w = mount(TimeChip, { props: { range: RANGE } });
    expect(w.get(".time-chip__text").text()).toBe("09:00 – 18:00");
  });

  it("非 editable → 无删除按钮", () => {
    const w = mount(TimeChip, { props: { range: RANGE } });
    expect(w.find('button[aria-label="删除时段"]').exists()).toBe(false);
  });

  it("editable → 点击删除 emit remove", async () => {
    const w = mount(TimeChip, { props: { range: RANGE, editable: true } });
    await w.get('button[aria-label="删除时段"]').trigger("click");
    expect(w.emitted("remove")).toHaveLength(1);
  });

  it("单击文本进入编辑 → 两个 time input（起止）+ 确认按钮", async () => {
    const w = mount(TimeChip, { props: { range: RANGE, editable: true } });
    await w.find(".time-chip__text").trigger("click");
    const inputs = w.findAll('input[type="time"]');
    expect(inputs).toHaveLength(2);
    expect(inputs[0]?.attributes("aria-label")).toBe("开始时间");
    expect(inputs[1]?.attributes("aria-label")).toBe("结束时间");
    expect(w.find('button[aria-label="完成"]').exists()).toBe(true);
  });

  it("改动起止后点 ✓ → emit update 带新 range 并退出编辑态", async () => {
    const w = mount(TimeChip, { props: { range: RANGE, editable: true } });
    await w.find(".time-chip__text").trigger("click");
    const start = w.get('input[aria-label="开始时间"]');
    const end = w.get('input[aria-label="结束时间"]');
    await start.setValue("08:30");
    await end.setValue("20:00");
    // 仅改值不退出编辑态
    expect(w.find('input[type="time"]').exists()).toBe(true);
    await w.get('button[aria-label="完成"]').trigger("click");
    const events = w.emitted("update");
    expect(events?.[0]).toEqual([{ start: "08:30", end: "20:00" }]);
    // 确认后退出编辑态
    expect(w.find('input[type="time"]').exists()).toBe(false);
  });

  it("autoEdit → 挂载即进入编辑态", async () => {
    const w = mount(TimeChip, {
      props: { range: RANGE, editable: true, autoEdit: true },
    });
    await nextTick();
    expect(w.findAll('input[type="time"]')).toHaveLength(2);
  });

  it("非 editable → 单击不进编辑", async () => {
    const w = mount(TimeChip, { props: { range: RANGE } });
    await w.find(".time-chip__text").trigger("click");
    expect(w.find('input[type="time"]').exists()).toBe(false);
  });
});

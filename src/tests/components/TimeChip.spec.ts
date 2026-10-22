// @vitest-environment happy-dom
/**
 * TimeChip 组件行为契约（F2.3 / D2 勿扰时段）：展示 start–end、
 * 可编辑变体行内改时段、删除事件；ui/ 红线——零业务依赖、仅受控事件上抛。
 */

import { mount } from "@vue/test-utils";
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

  it("双击进入编辑 → 两个 time input（起止）", async () => {
    const w = mount(TimeChip, { props: { range: RANGE, editable: true } });
    await w.find(".time-chip").trigger("dblclick");
    const inputs = w.findAll('input[type="time"]');
    expect(inputs).toHaveLength(2);
    expect(inputs[0]?.attributes("aria-label")).toBe("开始时间");
    expect(inputs[1]?.attributes("aria-label")).toBe("结束时间");
  });

  it("改动起始时间 → change emit update 带新 range", async () => {
    const w = mount(TimeChip, { props: { range: RANGE, editable: true } });
    await w.find(".time-chip").trigger("dblclick");
    const start = w.get('input[aria-label="开始时间"]');
    await start.setValue("08:30");
    await start.trigger("change");
    const events = w.emitted("update");
    expect(events?.[0]).toEqual([{ start: "08:30", end: "18:00" }]);
    // 提交后退出编辑态
    expect(w.find('input[type="time"]').exists()).toBe(false);
  });

  it("非 editable → 双击不进编辑", async () => {
    const w = mount(TimeChip, { props: { range: RANGE } });
    await w.find(".time-chip").trigger("dblclick");
    expect(w.find('input[type="time"]').exists()).toBe(false);
  });
});

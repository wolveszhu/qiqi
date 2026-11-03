// @vitest-environment happy-dom
/**
 * LevelJourney 组件契约：四档三态（已达成/进行中/未解锁）+ 档内进度计算。
 * 阈值与 `utils/affection` 同源，此处只验展示派生逻辑。
 */

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import LevelJourney from "../../components/affection/LevelJourney.vue";

describe("LevelJourney", () => {
  it("渲染四个等级台阶", () => {
    const w = mount(LevelJourney, { props: { points: 0, level: 1 } });
    expect(w.findAll(".jstep")).toHaveLength(4);
    expect(w.get(".jstep.is-current .jstep__name").text()).toContain("陌生");
  });

  it("状态分类：低于当前=已达成，等于=进行中，高于=未解锁", () => {
    const w = mount(LevelJourney, { props: { points: 600, level: 3 } });
    const cls = w.findAll(".jstep").map((e) => e.classes());
    expect(cls[0]).toContain("is-achieved");
    expect(cls[1]).toContain("is-achieved");
    expect(cls[2]).toContain("is-current");
    expect(cls[3]).toContain("is-locked");
  });

  it("当前档进度随 points 增加（边界 550→1050 档内）", () => {
    const mid = mount(LevelJourney, { props: { points: 800, level: 3 } });
    const fill = mid.get(".jstep.is-current .jstep__fill").attributes("style") ?? "";
    // 550..1050 档内，800 约在中段（48% 附近）
    expect(fill).toContain("width: 50%");
  });

  it("门槛文案对齐阈值（L2=200 / L3=550 / L4=1050，L1=起点）", () => {
    const w = mount(LevelJourney, { props: { points: 0, level: 1 } });
    const reqs = w.findAll(".jstep__req").map((e) => e.text());
    expect(reqs[0]).toBe("起点");
    expect(reqs[1]).toContain("200 分");
    expect(reqs[2]).toContain("550 分");
    expect(reqs[3]).toContain("1050 分");
  });
});

// @vitest-environment happy-dom
/**
 * AffectionRing 组件行为契约（F2.3）：角度随分数变化 + **数值仅在窗口 B 看板指定槽位**。
 *
 * 红线固化（冷眼三问②）：好感度数值的**唯一**可见路径是设置窗看板
 * （`.aff-ring__num` 槽位 + aria-label）；窗口 A 对话气泡由 SpeechBubble 单独测试断言
 * "只渲染传入文本、无任何数值注入"。此处回归拦截"圆环把数值写进可外溢节点"的改法。
 */

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import AffectionRing from "../../components/affection/AffectionRing.vue";
import { ringAngle } from "../../utils/affection";

/** 取 conic-gradient 中的角度（deg；首个 `Ndeg` 即进度角，注意 var(--peach) 含括号）。 */
function angleOf(wrapper: ReturnType<typeof mount>): number {
  const style = wrapper.get(".aff-ring").attributes("style") ?? "";
  expect(style).toContain("conic-gradient");
  const m = /(\d+(?:\.\d+)?)deg/.exec(style);
  expect(m, `style 应含 conic-gradient 角度: ${style}`).not.toBeNull();
  return Number(m?.[1]);
}

describe("AffectionRing", () => {
  it("0 分 → 0deg；1000 分 → 360deg", () => {
    expect(angleOf(mount(AffectionRing, { props: { points: 0 } }))).toBe(0);
    expect(angleOf(mount(AffectionRing, { props: { points: 1000 } }))).toBe(360);
  });

  it("角度随分数线性变化（200/201/500/750 边界）", () => {
    for (const p of [200, 201, 500, 750]) {
      expect(angleOf(mount(AffectionRing, { props: { points: p } }))).toBeCloseTo(ringAngle(p), 6);
    }
  });

  it("越界 clamp：负值→0deg，超限→360deg", () => {
    expect(angleOf(mount(AffectionRing, { props: { points: -50 } }))).toBe(0);
    expect(angleOf(mount(AffectionRing, { props: { points: 5000 } }))).toBe(360);
  });

  it("数值仅在指定槽位（.aff-ring__num）与 aria-label 暴露", () => {
    const w = mount(AffectionRing, { props: { points: 640 } });
    // 槽位文本恰为分值本身
    expect(w.get(".aff-ring__num").text()).toBe("640");
    // 无障碍标签带分值/总量
    expect(w.get(".aff-ring").attributes("aria-label")).toBe("好感度 640 / 1000");
    // 圆环容器直接子节点里不得再出现裸数值文本（防外溢到通用节点）
    const ringText = w.get(".aff-ring__inner").text();
    expect(ringText.replace(/\s+/g, "")).toBe("640of1000");
  });
});

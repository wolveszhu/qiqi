// @vitest-environment happy-dom
/**
 * QuotaBar 进度条契约（额度卡片真实数据源改造后回归）：
 * ratio 即「剩余比例」，宽度与 aria 值随比例变化；越界钳制到 0–1。
 */

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import QuotaBar from "../../components/settings/QuotaBar.vue";

describe("QuotaBar", () => {
  it("ratio=1 → 填充 100%", () => {
    const w = mount(QuotaBar, { props: { ratio: 1 } });
    const fill = w.get(".quota-bar__fill");
    expect(fill.attributes("style")).toContain("width: 100%");
    expect(w.get('[role="progressbar"]').attributes("aria-valuenow")).toBe("100");
  });

  it("ratio=0.35 → 填充 35%（剩余比例直通）", () => {
    const w = mount(QuotaBar, { props: { ratio: 0.35 } });
    expect(w.get(".quota-bar__fill").attributes("style")).toContain("width: 35%");
  });

  it("ratio 越界钳制到 [0,1]", () => {
    const high = mount(QuotaBar, { props: { ratio: 1.7 } });
    expect(high.get(".quota-bar__fill").attributes("style")).toContain("width: 100%");
    const low = mount(QuotaBar, { props: { ratio: -0.5 } });
    expect(low.get(".quota-bar__fill").attributes("style")).toContain("width: 0%");
  });
});

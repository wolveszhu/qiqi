import { describe, expect, it } from "vitest";

import { chooseSide, resolveBubbleLayout, SAFETY_MARGIN } from "../../utils/bubbleLayout";
import type { HitRegion } from "../../api/types";

const WIN_W = 400;
const cat = (x: number, w = 200): HitRegion => ({ x, y: 100, width: w, height: 200 });

describe("chooseSide", () => {
  it("右侧空间充足 → 优先 right", () => {
    // 猫在左侧（x=20），右侧空间充足
    expect(chooseSide(WIN_W, cat(20), 200)).toBe("right");
  });

  it("右侧空间不足 → 翻 left", () => {
    // 猫贴右缘（x=320），右侧空间仅 80 < 气泡 200 → 左侧
    expect(chooseSide(WIN_W, cat(320), 200)).toBe("left");
  });

  it("贴边（猫居中右）两侧都不够 → 选较宽侧", () => {
    // 猫 x=300 宽 200 → 右空间 0，左空间 300 → left
    expect(chooseSide(WIN_W, cat(300, 200), 250)).toBe("left");
  });

  it("滞回：prevSide=right 且略有不足但余量≥滞回时不翻", () => {
    // 右侧剩余 200-? 场景：猫 x=120 宽 200 → 右空间 80；气泡 60 → 右够(80-60=20>=8+24? 否)
    // 用 prevSide=right 且右空间 - 气泡宽 >= 8+24 才保持
    const region = cat(40, 200); // 右空间 160，气泡 120 → 160-120=40 >= 8+24=32 保持 right
    expect(chooseSide(WIN_W, region, 120, "right")).toBe("right");
  });

  it("滞回：右侧明显不足则翻 left（即使 prevSide=right）", () => {
    const region = cat(250, 200); // 右空间 150? 不，x=250 宽200 → catRight=450 超出窗口；右空间=400-450<0
    expect(chooseSide(WIN_W, region, 120, "right")).toBe("left");
  });
});

describe("resolveBubbleLayout", () => {
  it("右侧场景：x 在猫右缘外 + maxWidth 不超过右侧空间", () => {
    const r = resolveBubbleLayout({ winW: WIN_W, catRegion: cat(20), bubbleW: 200 });
    expect(r.side).toBe("right");
    expect(r.x).toBe(20 + 200 + SAFETY_MARGIN);
    expect(r.maxWidth).toBeLessThanOrEqual(WIN_W - r.x - SAFETY_MARGIN + 1);
  });

  it("左侧场景：x clamp 不溢出左缘", () => {
    const r = resolveBubbleLayout({ winW: WIN_W, catRegion: cat(360, 20), bubbleW: 200 });
    expect(r.side).toBe("left");
    expect(r.x).toBeGreaterThanOrEqual(SAFETY_MARGIN);
  });

  it("maxWidth 永不小于安全边距（防 0 宽渲染）", () => {
    const r = resolveBubbleLayout({ winW: WIN_W, catRegion: cat(380, 10), bubbleW: 380 });
    expect(r.maxWidth).toBeGreaterThanOrEqual(SAFETY_MARGIN);
  });
});

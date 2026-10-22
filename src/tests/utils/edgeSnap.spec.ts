import { describe, expect, it } from "vitest";

import { detectSnapEdge, isSnapped, SNAP_THRESHOLD } from "../../utils/edgeSnap";

// 显示器 1920x1080 左上 (0,0)
const MON = { x: 0, y: 0, width: 1920, height: 1080 };

describe("detectSnapEdge", () => {
  it("top 命中（y 贴近上缘）", () => {
    expect(detectSnapEdge({ x: 960, y: 10 }, MON)).toBe("top");
    expect(detectSnapEdge({ x: 960, y: MON.y + SNAP_THRESHOLD }, MON)).toBe("top");
  });

  it("left 命中（x 贴近左缘，非上缘）", () => {
    expect(detectSnapEdge({ x: 5, y: 500 }, MON)).toBe("left");
  });

  it("right 命中（x 贴近右缘）", () => {
    expect(detectSnapEdge({ x: MON.width - 5, y: 500 }, MON)).toBe("right");
  });

  it("阈值边界：恰好在阈值外 → null", () => {
    // top 边界外 1px
    expect(detectSnapEdge({ x: 960, y: MON.y + SNAP_THRESHOLD + 1 }, MON)).toBeNull();
    // left 边界外 1px
    expect(detectSnapEdge({ x: MON.x + SNAP_THRESHOLD + 1, y: 500 }, MON)).toBeNull();
  });

  it("双角优先级：top+left 同时命中 → top 优先", () => {
    expect(detectSnapEdge({ x: 3, y: 3 }, MON)).toBe("top");
    // top+right 同角
    expect(detectSnapEdge({ x: MON.width - 3, y: 3 }, MON)).toBe("top");
  });

  it("不命中 → null", () => {
    expect(detectSnapEdge({ x: 960, y: 540 }, MON)).toBeNull();
  });

  it("多屏负坐标（主屏左侧副屏）", () => {
    const leftMon = { x: -1920, y: 0, width: 1920, height: 1080 };
    expect(detectSnapEdge({ x: -1915, y: 500 }, leftMon)).toBe("left");
    expect(detectSnapEdge({ x: -1920 + 5, y: 5 }, leftMon)).toBe("top");
  });
});

describe("isSnapped", () => {
  it("贴边为真、游离为假", () => {
    expect(isSnapped({ x: 5, y: 500 }, MON)).toBe(true);
    expect(isSnapped({ x: 960, y: 540 }, MON)).toBe(false);
  });
});

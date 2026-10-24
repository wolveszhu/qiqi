import { describe, expect, it } from "vitest";

import { detectSnapEdge, isSnapped, SNAP_THRESHOLD } from "../../utils/edgeSnap";

// 显示器 1920x1080 左上 (0,0)；窗口 150x150（常态贴合尺寸）
const MON = { x: 0, y: 0, width: 1920, height: 1080 };
const WIN = { width: 150, height: 150 };

describe("detectSnapEdge（仅左右吸附；上下不吸附）", () => {
  it("left 命中（窗口左缘贴近显示器左缘）", () => {
    expect(detectSnapEdge({ x: 5, y: 500, ...WIN }, MON)).toBe("left");
    expect(detectSnapEdge({ x: MON.x + SNAP_THRESHOLD, y: 500, ...WIN }, MON)).toBe(
      "left",
    );
  });

  it("right 命中（窗口**右缘** x+width 贴近显示器右缘）", () => {
    // 右缘贴边时窗口左缘 = 1920-150 = 1770（远大于阈值 32——旧实现此处漏判）
    expect(detectSnapEdge({ x: MON.width - WIN.width, y: 500, ...WIN }, MON)).toBe(
      "right",
    );
    expect(
      detectSnapEdge({ x: MON.width - WIN.width - SNAP_THRESHOLD, y: 500, ...WIN }, MON),
    ).toBe("right");
  });

  it("top/bottom 不吸附（用户决策：仅左右）", () => {
    // 窗口顶到上缘 / 底缘 → null
    expect(detectSnapEdge({ x: 960, y: 0, ...WIN }, MON)).toBeNull();
    expect(
      detectSnapEdge({ x: 960, y: MON.height - WIN.height, ...WIN }, MON),
    ).toBeNull();
  });

  it("阈值边界：恰好在阈值外 → null", () => {
    // left 边界外 1px
    expect(
      detectSnapEdge({ x: MON.x + SNAP_THRESHOLD + 1, y: 500, ...WIN }, MON),
    ).toBeNull();
    // right 边界外 1px（右缘距右缘 > 阈值）
    expect(
      detectSnapEdge(
        { x: MON.width - WIN.width - SNAP_THRESHOLD - 1, y: 500, ...WIN },
        MON,
      ),
    ).toBeNull();
  });

  it("不命中（屏幕中央）→ null", () => {
    expect(detectSnapEdge({ x: 960, y: 540, ...WIN }, MON)).toBeNull();
  });

  it("多屏负坐标（主屏左侧副屏）", () => {
    const leftMon = { x: -1920, y: 0, width: 1920, height: 1080 };
    expect(detectSnapEdge({ x: -1915, y: 500, ...WIN }, leftMon)).toBe("left");
    expect(
      detectSnapEdge({ x: -1920 + leftMon.width - WIN.width, y: 500, ...WIN }, leftMon),
    ).toBe("right");
  });
});

describe("isSnapped", () => {
  it("贴边为真、游离为假", () => {
    expect(isSnapped({ x: 5, y: 500, ...WIN }, MON)).toBe(true);
    expect(isSnapped({ x: 960, y: 540, ...WIN }, MON)).toBe(false);
  });
});

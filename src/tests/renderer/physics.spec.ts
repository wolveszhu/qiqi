import { describe, expect, it } from "vitest";

import {
  breathScales,
  dampedRecovery,
  eyeOffset,
  jellyImpulse,
  rotatedHitRegion,
  stretchFactor,
} from "../../renderer/physics";

describe("breathScales", () => {
  it("t=0 时无扰动", () => {
    expect(breathScales(0)).toEqual([1, 1]);
  });

  it("幅值对称且受 amplitude 控制", () => {
    const [sx, sy] = breathScales(Math.PI / 2, 0.04);
    expect(sx).toBeCloseTo(1 - 0.02, 10);
    expect(sy).toBeCloseTo(1 + 0.04, 10);
  });
});

describe("dampedRecovery", () => {
  it("从 1.15 单调收敛到 1（60 帧误差 < 0.01）", () => {
    let value = 1.15;
    for (let i = 0; i < 60; i += 1) {
      value = dampedRecovery(value);
      expect(value).toBeLessThanOrEqual(1.15);
    }
    expect(Math.abs(value - 1)).toBeLessThan(0.01);
  });

  it("低于 1 时向上回弹", () => {
    expect(dampedRecovery(0.85)).toBeGreaterThan(0.85);
  });
});

describe("stretchFactor", () => {
  it("小位移线性放大", () => {
    expect(stretchFactor(50, 0)).toBeCloseTo(0.5, 10);
  });

  it("上限 1.5 封顶", () => {
    expect(stretchFactor(150, 0)).toBe(1.5);
    expect(stretchFactor(300, 400)).toBe(1.5);
  });
});

describe("eyeOffset", () => {
  it("远距离目标偏移限幅 5px", () => {
    const { dx, dy } = eyeOffset(1000, 1000, 0, 0, 5);
    expect(Math.hypot(dx, dy)).toBeCloseTo(5, 10);
  });

  it("零距离返回 0（除零保护）", () => {
    expect(eyeOffset(0, 0, 0, 0, 5)).toEqual({ dx: 0, dy: 0 });
  });
});

describe("jellyImpulse", () => {
  it("恒为拉宽压扁冲击 [1.15, 0.85]", () => {
    expect(jellyImpulse()).toEqual([1.15, 0.85]);
  });
});

describe("rotatedHitRegion", () => {
  // 基准盒：窗口 400×400，中心 (200,200)，w=100 h=50，left=-50 top=-25（盒中心=旋转中心）
  const BASE = [200, 200, 100, 50, -50, -25] as const;

  it("0° 恒等（盒中心与旋转中心重合）", () => {
    expect(rotatedHitRegion(...BASE, 0, 400, 400)).toEqual({
      x: 150,
      y: 175,
      width: 100,
      height: 50,
    });
  });

  it("90° 宽高互换（100×50 → 50×100）", () => {
    expect(rotatedHitRegion(...BASE, Math.PI / 2, 400, 400)).toEqual({
      x: 175,
      y: 150,
      width: 50,
      height: 100,
    });
  });

  it("45° AABB 半宽 = |w/2·cos| + |h/2·sin|", () => {
    const r = rotatedHitRegion(...BASE, Math.PI / 4, 400, 400);
    const k = Math.SQRT1_2;
    expect(r.width).toBeCloseTo(100 * k + 50 * k, 10);
    expect(r.height).toBeCloseTo(100 * k + 50 * k, 10);
    // 旋转中心重合 → AABB 中心仍在 (200,200)
    expect(r.x + r.width / 2).toBeCloseTo(200, 10);
    expect(r.y + r.height / 2).toBeCloseTo(200, 10);
  });

  it("盒中心偏离旋转中心时随旋转平移（rendered = P + R(θ)·p）", () => {
    // 盒中心局部 (-50, 50)，转 90° → (-50, -50)
    const r = rotatedHitRegion(200, 200, 100, 100, -100, 0, Math.PI / 2, 400, 400);
    expect(r.x + r.width / 2).toBeCloseTo(150, 10);
    expect(r.y + r.height / 2).toBeCloseTo(150, 10);
  });

  it("贴边躺倒溢出窗口的部分被裁切", () => {
    // 中心贴左边缘 (20,200)：直立盒左半溢出 → 从窗口 0 起裁
    expect(rotatedHitRegion(20, 200, 100, 50, -50, -25, 0, 400, 400)).toEqual({
      x: 0,
      y: 175,
      width: 70,
      height: 50,
    });
  });

  it("整盒横向溢出窗口 → 该轴裁成 0 尺寸（交由 ensureValidHitRegion 兜底整窗）", () => {
    // 中心 (-200,200)：x 方向完全在窗口外 → width 裁 0；y 方向仍可见
    const r = rotatedHitRegion(-200, 200, 100, 50, -50, -25, 0, 400, 400);
    expect(r.x).toBe(0);
    expect(r.width).toBe(0);
    expect(r.height).toBe(50);
  });
});

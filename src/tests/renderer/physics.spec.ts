import { describe, expect, it } from "vitest";

import {
  breathScales,
  dampedRecovery,
  eyeOffset,
  jellyImpulse,
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

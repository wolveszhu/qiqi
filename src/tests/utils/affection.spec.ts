/**
 * 好感度展示映射单测（F2.2 / 勘误 #20#1：前端仅"展示映射"，规则引擎在 Rust F1 已测）。
 *
 * 覆盖：等级→文案、下一档 diff/sub（0/200/201/500/750/1000 边界）、
 * 圆环百分比/角度（clamp 负值与超限）。
 */

import { describe, expect, it } from "vitest";

import {
  AFFECTION_MAX,
  LEVEL_THRESHOLDS,
  levelLabel,
  nextLevelInfo,
  ringAngle,
  ringPct,
} from "../../utils/affection";

describe("levelLabel", () => {
  it("1–4 映射 baseline §F 命名", () => {
    expect(levelLabel(1)).toBe("陌生");
    expect(levelLabel(2)).toBe("熟人");
    expect(levelLabel(3)).toBe("亲密");
    expect(levelLabel(4)).toBe("羁绊");
  });

  it("越界等级回退 Lv.N（0 与 5+）", () => {
    expect(levelLabel(0)).toBe("Lv.0");
    expect(levelLabel(5)).toBe("Lv.5");
    expect(levelLabel(99)).toBe("Lv.99");
  });
});

describe("nextLevelInfo", () => {
  it("Lv1 距下一档（阈值 250）差值正确", () => {
    expect(nextLevelInfo(1, 0)).toEqual({ diff: 250, sub: "熟悉了，会主动搭话" });
    expect(nextLevelInfo(1, 200)).toEqual({ diff: 50, sub: "熟悉了，会主动搭话" });
    expect(nextLevelInfo(1, 201)).toEqual({ diff: 49, sub: "熟悉了，会主动搭话" });
  });

  it("Lv2/Lv3 距下一档（阈值 500/750）差值正确", () => {
    expect(nextLevelInfo(2, 250)).toEqual({ diff: 250, sub: "亲密，会撒娇会想念" });
    expect(nextLevelInfo(3, 500)).toEqual({ diff: 250, sub: "羁绊，形影不离" });
  });

  it("恰好在下一档阈值 → diff 归零", () => {
    expect(nextLevelInfo(2, 500).diff).toBe(0);
    expect(nextLevelInfo(3, 750).diff).toBe(0);
  });

  it("已越过下一档阈值 → diff 归零（不报负）", () => {
    expect(nextLevelInfo(1, 500).diff).toBe(0);
    expect(nextLevelInfo(2, 999).diff).toBe(0);
  });

  it("Lv4 顶格：diff=0 + 顶格文案", () => {
    expect(nextLevelInfo(4, 1000)).toEqual({ diff: 0, sub: "已抵达最高羁绊" });
    expect(nextLevelInfo(9, 0)).toEqual({ diff: 0, sub: "已抵达最高羁绊" });
  });

  it("阈值表自洽：升序且首 0 尾 1000", () => {
    expect(LEVEL_THRESHOLDS[0]).toBe(0);
    expect(LEVEL_THRESHOLDS[LEVEL_THRESHOLDS.length - 1]).toBe(AFFECTION_MAX);
  });
});

describe("ringPct / ringAngle", () => {
  it("边界值 0 / 500 / 1000 → 0 / 0.5 / 1", () => {
    expect(ringPct(0)).toBe(0);
    expect(ringPct(500)).toBeCloseTo(0.5, 10);
    expect(ringPct(1000)).toBe(1);
  });

  it("角度 = 百分比 × 360（200/201 边界）", () => {
    expect(ringAngle(0)).toBe(0);
    expect(ringAngle(1000)).toBe(360);
    expect(ringAngle(500)).toBeCloseTo(180, 10);
    expect(ringAngle(200)).toBeCloseTo(72, 10);
    expect(ringAngle(201)).toBeCloseTo(72.36, 10);
  });

  it("越界 clamp：负值→0，超限→满", () => {
    expect(ringPct(-50)).toBe(0);
    expect(ringAngle(-50)).toBe(0);
    expect(ringPct(5000)).toBe(1);
    expect(ringAngle(5000)).toBe(360);
  });
});

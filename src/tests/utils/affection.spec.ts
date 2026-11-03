/**
 * 好感度展示映射单测（F2.2 / 勘误 #20#1：前端仅"展示映射"，规则引擎在 Rust F1 已测）。
 *
 * 覆盖：等级→文案、下一档 diff/sub（0/200/550/1050/2000 边界）、
 * 圆环百分比/角度（clamp 负值与超限）。
 */

import { describe, expect, it } from "vitest";

import {
  AFFECTION_MAX,
  LEVEL_THRESHOLDS,
  levelLabel,
  nextLevelInfo,
  reasonMeta,
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
  it("Lv1 距下一档（档上沿 200）差值正确", () => {
    expect(nextLevelInfo(1, 0)).toEqual({ diff: 200, sub: "熟悉了，会主动搭话" });
    expect(nextLevelInfo(1, 200)).toEqual({ diff: 0, sub: "熟悉了，会主动搭话" });
    expect(nextLevelInfo(1, 201)).toEqual({ diff: 0, sub: "熟悉了，会主动搭话" });
  });

  it("Lv2/Lv3 距下一档（档上沿 550/1050）差值正确", () => {
    expect(nextLevelInfo(2, 250)).toEqual({ diff: 300, sub: "亲密，会撒娇会想念" });
    expect(nextLevelInfo(3, 500)).toEqual({ diff: 550, sub: "羁绊，形影不离" });
  });

  it("恰好在下一档阈值 → diff 归零", () => {
    expect(nextLevelInfo(2, 550).diff).toBe(0);
    expect(nextLevelInfo(3, 1050).diff).toBe(0);
  });

  it("已越过下一档阈值 → diff 归零（不报负）", () => {
    expect(nextLevelInfo(1, 550).diff).toBe(0);
    expect(nextLevelInfo(2, 1999).diff).toBe(0);
  });

  it("Lv4 顶格：diff=0 + 顶格文案", () => {
    expect(nextLevelInfo(4, 2000)).toEqual({ diff: 0, sub: "已抵达最高羁绊" });
    expect(nextLevelInfo(9, 0)).toEqual({ diff: 0, sub: "已抵达最高羁绊" });
  });

  it("阈值表自洽：升序且首 0 尾 = AFFECTION_MAX(2000)", () => {
    expect(LEVEL_THRESHOLDS[0]).toBe(0);
    expect(LEVEL_THRESHOLDS[LEVEL_THRESHOLDS.length - 1]).toBe(AFFECTION_MAX);
  });

  it("阈值梯度上升：各档间隔 200<350<500（用户裁决硬性值）", () => {
    const gaps = LEVEL_THRESHOLDS.slice(1).map((v, i) => v - LEVEL_THRESHOLDS[i]);
    expect(gaps).toEqual([200, 350, 500, 950]);
    // 前三级间隔必须严格递增，杜绝改回等距或倒挂
    expect(gaps.slice(0, 3).every((g, i, a) => i === 0 || g > a[i - 1])).toBe(true);
  });
});

describe("ringPct / ringAngle", () => {
  it("边界值 0 / 1000 / 2000 → 0 / 0.5 / 1", () => {
    expect(ringPct(0)).toBe(0);
    expect(ringPct(1000)).toBeCloseTo(0.5, 10);
    expect(ringPct(2000)).toBe(1);
  });

  it("角度 = 百分比 × 360（200/201 边界）", () => {
    expect(ringAngle(0)).toBe(0);
    expect(ringAngle(2000)).toBe(360);
    expect(ringAngle(1000)).toBeCloseTo(180, 10);
    expect(ringAngle(200)).toBeCloseTo(36, 10);
    expect(ringAngle(201)).toBeCloseTo(36.18, 10);
  });

  it("越界 clamp：负值→0，超限→满", () => {
    expect(ringPct(-50)).toBe(0);
    expect(ringAngle(-50)).toBe(0);
    expect(ringPct(5000)).toBe(1);
    expect(ringAngle(5000)).toBe(360);
  });
});

describe("reasonMeta", () => {
  it("后端英文 key → 中文文案（覆盖 reason.rs 全常量）", () => {
    expect(reasonMeta("daily_check_in").label).toBe("每日打卡");
    expect(reasonMeta("pet_stroke").label).toBe("抚摸");
    expect(reasonMeta("deep_chat").label).toBe("深度对话");
    expect(reasonMeta("resonance").label).toBe("情感共鸣");
    expect(reasonMeta("taboo_violation").label).toBe("触犯禁忌");
    expect(reasonMeta("long_absence").label).toBe("久未互动衰减");
  });

  it("扣分/衰减类带对应 emoji", () => {
    expect(reasonMeta("taboo_violation").emoji).toBe("😾");
    expect(reasonMeta("long_absence").emoji).toBe("💤");
    expect(reasonMeta("daily_check_in").emoji).toBe("🌞");
  });

  it("未知 key 原样回退（不臆造文案，便于排查新信号）", () => {
    expect(reasonMeta("some_new_signal")).toEqual({
      label: "some_new_signal",
      emoji: "✨",
    });
    expect(reasonMeta("")).toEqual({ label: "", emoji: "✨" });
  });
});

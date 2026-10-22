/**
 * 主动气泡节奏策略单测（F2.2 / E3.3）：`bubbleTtlMs` 按字数定显示时长。
 *
 * 策略：min 2s / +0.05s·字 / 上限 12s；禁忌(angry)与傲娇(tsundere)话术停留 ×1.5。
 */

import { describe, expect, it } from "vitest";

import { bubbleTtlMs } from "../../utils/bubbleRhythm";

describe("bubbleTtlMs", () => {
  it("短句（≤0 字基线）不低于 2s 下限", () => {
    expect(bubbleTtlMs("", "happy")).toBe(2000);
    expect(bubbleTtlMs("喵", "happy")).toBe(2050);
  });

  it("线性增长 2000 + 50ms·字", () => {
    // 20 字 → 2000 + 1000 = 3000
    const text = "啊".repeat(20);
    expect(bubbleTtlMs(text, "happy")).toBe(3000);
    // 40 字 → 4000
    expect(bubbleTtlMs("啊".repeat(40), "happy")).toBe(4000);
  });

  it("超长话术封顶 12s", () => {
    expect(bubbleTtlMs("啊".repeat(400), "happy")).toBe(12_000);
  });

  it("多字节字符按码点计数（非 UTF-16 长度）", () => {
    // emoji 为代理对：按 [...text] 码点计 1 字
    expect(bubbleTtlMs("🐱", "happy")).toBe(2050);
  });

  it("禁忌/傲娇话术停留加长 1.5×", () => {
    const text = "啊".repeat(20); // 基线 3000
    expect(bubbleTtlMs(text, "angry")).toBe(4500);
    expect(bubbleTtlMs(text, "tsundere")).toBe(4500);
  });

  it("加长后仍受 12s 上限约束", () => {
    // 基线已封顶 12000 → ×1.5 后仍 clamp 回 12000
    expect(bubbleTtlMs("啊".repeat(400), "angry")).toBe(12_000);
  });

  it("普通情绪不加长（happy/neutral）", () => {
    const text = "啊".repeat(20);
    expect(bubbleTtlMs(text, "neutral")).toBe(3000);
    expect(bubbleTtlMs(text, "sleepy")).toBe(3000);
  });
});

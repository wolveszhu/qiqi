/**
 * 微随机引擎单测（C6.3 / F2）：预算区间、勿拒判定、降频 ×0.2、
 * 锚点窗口含边界、傲娇 72h 边界、rng 注入确定性。
 */

import { describe, expect, it } from "vitest";

import {
  ANCHOR_WINDOWS,
  MIN_FIRE_INTERVAL_MS,
  TSUNDERE_DAYS,
} from "../../constants/proactive";
import {
  dailyBudget,
  inAnchorWindow,
  isTsundere,
  matchWorkProcess,
  shouldFire,
  type ProactiveCtx,
} from "../../utils/microRandom";

/** 确定性 rng：固定返回值序列（循环取用）。 */
function seqRng(...values: number[]): () => number {
  let i = 0;
  return () => values[i++ % values.length] ?? 0;
}

/** 2026-10-22 周四 08:30（本地）毫秒。 */
const MORNING_ANCHOR = new Date(2026, 9, 22, 8, 30, 0).getTime();

function baseCtx(over: Partial<ProactiveCtx> = {}): ProactiveCtx {
  return {
    now: MORNING_ANCHOR,
    tier: "normal",
    dndEnabled: false,
    inDndRange: false,
    foreground: null,
    budgetUsed: 0,
    lastFireAt: null,
    ...over,
  };
}

describe("dailyBudget", () => {
  it("quiet 档抽取结果恒在 1-2", () => {
    const rng = seqRng(0, 0.499, 0.999);
    expect(dailyBudget("quiet", rng)).toBe(1);
    expect(dailyBudget("quiet", rng)).toBe(1);
    expect(dailyBudget("quiet", rng)).toBe(2);
  });

  it("normal 档区间 4-6（rng=0 取下界，rng≈1 取上界）", () => {
    expect(dailyBudget("normal", seqRng(0))).toBe(4);
    expect(dailyBudget("normal", seqRng(0.999))).toBe(6);
  });

  it("sticky 档区间 10-14", () => {
    expect(dailyBudget("sticky", seqRng(0))).toBe(10);
    expect(dailyBudget("sticky", seqRng(0.999))).toBe(14);
  });
});

describe("inAnchorWindow", () => {
  it("08:30 整点命中（含起始边界）", () => {
    expect(inAnchorWindow(MORNING_ANCHOR)).toBe(true);
  });

  it("09:30 截止（含结束边界）", () => {
    const end = new Date(2026, 9, 22, 9, 30, 0).getTime();
    expect(inAnchorWindow(end)).toBe(true);
  });

  it("09:31 与 08:29 均不命中", () => {
    expect(inAnchorWindow(new Date(2026, 9, 22, 9, 31).getTime())).toBe(false);
    expect(inAnchorWindow(new Date(2026, 9, 22, 8, 29).getTime())).toBe(false);
  });

  it("22:00-23:30 深夜窗口命中", () => {
    expect(inAnchorWindow(new Date(2026, 9, 22, 23, 0).getTime())).toBe(true);
    expect(inAnchorWindow(new Date(2026, 9, 22, 23, 31).getTime())).toBe(false);
  });

  it("默认锚点表非空且时间有序", () => {
    expect(ANCHOR_WINDOWS.length).toBeGreaterThanOrEqual(2);
    for (const a of ANCHOR_WINDOWS) {
      expect(a.startMin).toBeLessThanOrEqual(a.endMin);
    }
  });
});

describe("isTsundere", () => {
  const now = new Date(2026, 9, 22, 12, 0, 0).getTime();
  const day = 24 * 60 * 60 * 1000;

  it("无互动记录 → false", () => {
    expect(isTsundere(null, now)).toBe(false);
  });

  it("恰好 72h（3 天）达阈值", () => {
    expect(isTsundere(now - TSUNDERE_DAYS * day, now)).toBe(true);
  });

  it("72h 差 1ms 未达阈值", () => {
    expect(isTsundere(now - TSUNDERE_DAYS * day + 1, now)).toBe(false);
  });

  it("5 天未互动 → true", () => {
    expect(isTsundere(now - 5 * day, now)).toBe(true);
  });
});

describe("matchWorkProcess", () => {
  it("名单内进程（大小写不敏感包含匹配）", () => {
    expect(matchWorkProcess("Code.exe")).toBe(true);
    expect(matchWorkProcess("idea64.exe")).toBe(true);
    expect(matchWorkProcess("steamwebhelper.exe")).toBe(true);
  });

  it("非工作场景：普通浏览器/桌面/无前台", () => {
    expect(matchWorkProcess("msedge.exe")).toBe(false);
    expect(matchWorkProcess("explorer.exe")).toBe(false);
    expect(matchWorkProcess(null)).toBe(false);
  });
});

describe("shouldFire", () => {
  it("勿扰开启且在时段内 → 恒 false（最高优先）", () => {
    const ctx = baseCtx({ dndEnabled: true, inDndRange: true });
    expect(shouldFire(ctx, seqRng(0.01))).toBe(false);
  });

  it("勿扰开启但不在时段内 → 可触发", () => {
    const ctx = baseCtx({ dndEnabled: true, inDndRange: false });
    expect(shouldFire(ctx, seqRng(0.01))).toBe(true);
  });

  it("最小间隔 45min 内 → false；恰好 45min 后 → 可触发", () => {
    const firedAt = MORNING_ANCHOR;
    expect(
      shouldFire(
        baseCtx({ lastFireAt: firedAt, now: firedAt + MIN_FIRE_INTERVAL_MS - 1 }),
        seqRng(0.01),
      ),
    ).toBe(false);
    expect(
      shouldFire(
        baseCtx({ lastFireAt: firedAt, now: firedAt + MIN_FIRE_INTERVAL_MS }),
        seqRng(0.01),
      ),
    ).toBe(true);
  });

  it("预算用尽（budgetUsed ≥ 抽取值）→ false", () => {
    // rng=0 → normal 抽 4；budgetUsed=4 → 用尽
    const rng = seqRng(0, 0.01);
    expect(shouldFire(baseCtx({ budgetUsed: 4 }), rng)).toBe(false);
  });

  it("工作进程 → 概率 ×0.2（低 rng 才触发；非锚点时段）", () => {
    const noon = new Date(2026, 9, 22, 14, 0, 0).getTime();
    const work = baseCtx({ now: noon, foreground: "Code.exe" });
    // 工作进程 prob=0.35×0.2=0.07：rng=0.05 触发、0.5 不触发
    expect(shouldFire(work, seqRng(0, 0.05))).toBe(true);
    expect(shouldFire(work, seqRng(0, 0.5))).toBe(false);
  });

  it("锚点窗口内强制高概率（0.9）", () => {
    // now 在锚点内（MORNING_ANCHOR）：rng=0.5 仍触发（0.5 < 0.9）
    expect(shouldFire(baseCtx(), seqRng(0, 0.5))).toBe(true);
    // rng=0.95 超过 0.9 不触发
    expect(shouldFire(baseCtx(), seqRng(0, 0.95))).toBe(false);
  });

  it("非锚点、非工作：基础概率 0.35（rng 注入确定性）", () => {
    const noon = new Date(2026, 9, 22, 14, 0, 0).getTime();
    expect(shouldFire(baseCtx({ now: noon }), seqRng(0, 0.3))).toBe(true);
    expect(shouldFire(baseCtx({ now: noon }), seqRng(0, 0.5))).toBe(false);
  });

  it("锚点当日已触发过（anchorBoostSuppressed）→ 回基础概率 0.35", () => {
    expect(shouldFire(baseCtx({ anchorBoostSuppressed: true }), seqRng(0, 0.5))).toBe(
      false,
    );
    expect(shouldFire(baseCtx({ anchorBoostSuppressed: true }), seqRng(0, 0.3))).toBe(
      true,
    );
  });

  it("rng 注入确定性：同输入同输出", () => {
    const ctx = baseCtx();
    expect(shouldFire(ctx, seqRng(0, 0.1))).toBe(shouldFire({ ...ctx }, seqRng(0, 0.1)));
  });
});

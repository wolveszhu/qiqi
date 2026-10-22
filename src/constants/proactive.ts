/**
 * 主动互动常量（C6，单一真源）：
 * 锚点窗口 / 档位预算区间 / 工作进程名单 / 节流参数。
 *
 * 与 Rust 侧无重复定义——Rust 仅保留系统层阈值（IDLE_THRESHOLD_MS），
 * 判定类常量集中于此（与 utils/microRandom.ts 同源，Vitest 注入 now/rng 可测）。
 */

import type { ProactiveFrequency } from "../api/types";

/** 主动互动编排 tick 间隔（毫秒；纯计算零 I/O，性能红线）。 */
export const PROACTIVE_TICK_MS = 5 * 60 * 1000;

/** 两次主动发言最小间隔（毫秒；45min，防刷屏三重约束之一）。 */
export const MIN_FIRE_INTERVAL_MS = 45 * 60 * 1000;

/** 傲娇判定：距上次互动 ≥ 此天数（与 Rust prompt::TSUNDERE_DAYS 同口径）。 */
export const TSUNDERE_DAYS = 3;

/** 锚点窗口（一天内分钟数；含起止边界——08:30 整点命中、09:30 截止）。 */
export interface AnchorWindow {
  /** 起始分钟（自 00:00 起）。 */
  startMin: number;
  /** 截止分钟（含）。 */
  endMin: number;
}

/** 时间窗口锚点：早间问候 / 深夜陪伴（裁决 §一 #2）。 */
export const ANCHOR_WINDOWS: AnchorWindow[] = [
  { startMin: 8 * 60 + 30, endMin: 9 * 60 + 30 }, // 08:30-09:30
  { startMin: 22 * 60, endMin: 23 * 60 + 30 }, // 22:00-23:30
];

/** 档位日预算区间（含边界；每日均匀抽取一次）。 */
export const TIER_BUDGET: Record<ProactiveFrequency, readonly [number, number]> = {
  quiet: [1, 2],
  normal: [4, 6],
  sticky: [10, 14],
};

/** 每 tick 基础触发概率（5min tick 下与 45min 最小间隔共同约束总量分布）。 */
export const BASE_FIRE_PROBABILITY = 0.35;

/** 锚点窗口内强制高概率（裁决：锚点当日去重 + 问候优先）。 */
export const ANCHOR_FIRE_PROBABILITY = 0.9;

/** 工作/游戏进程命中时的概率乘数（×0.2，"不打扰工作"核心卖点）。 */
export const WORK_PROCESS_MULTIPLIER = 0.2;

/**
 * 工作/游戏进程名单（**进程名包含匹配**，小写比较；仅进程名，绝不读窗口内容）。
 * MVP 常量（D2 Phase 2 可视可编辑）。
 */
export const WORK_PROCESS_NAMES: readonly string[] = [
  // 办公/开发
  "code.exe",
  "cursor.exe",
  "idea64.exe",
  "pycharm64.exe",
  "webstorm64.exe",
  "clion64.exe",
  "goland64.exe",
  "rider64.exe",
  "devenv.exe",
  "studio64.exe",
  "wps.exe",
  "et.exe",
  "winword.exe",
  "excel.exe",
  "powerpnt.exe",
  "outlook.exe",
  // 办公通讯
  "wxwork.exe",
  "dingtalk.exe",
  "feishu.exe",
  // 游戏/启动器
  "steam.exe",
  "steamwebhelper.exe",
  "epicgameslauncher.exe",
  "battle.net.exe",
  "origin.exe",
  "xboxapp.exe",
  "leagueclient.exe",
  "league of legends.exe",
  "cs2.exe",
  "dota2.exe",
  "genshinimpact.exe",
  "star rail.exe",
  "zenlesszonezero.exe",
  "minecraft.exe",
  "javaw.exe",
];

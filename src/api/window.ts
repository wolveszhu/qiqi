/**
 * window 命令封装（C2 桌面融合；规范 §3：唯一 invoke 出口 = `callCommand`）。
 *
 * 纪律：系统能力一律 Rust 实现（rule.md §5）——前端只做调用与反馈，
 * 禁止自行模拟拖拽/穿透。
 *
 * 参数约定（Tauri 2）：invoke 按**参数名**取值，见各命令 Rust 签名。
 */

import { callCommand } from "./client";
import type { HitRegion } from "./types";

/** 开始拖拽窗口 A（系统接管：移动/跨屏/DPI 均由 OS 处理）。 */
export async function startDrag(): Promise<void> {
  await callCommand("window_start_drag");
}

/** 显隐窗口（label：main=桌宠层 / settings=设置层；关闭=hide 由 Rust 拦截）。 */
export async function show(target: "main" | "settings"): Promise<void> {
  await callCommand("window_show", { target });
}

export async function hide(target: "main" | "settings"): Promise<void> {
  await callCommand("window_hide", { target });
}

/** 上报命中区域（窗口局部坐标；变化才调，约 500ms 低频比对）。 */
export async function setHitRegion(region: HitRegion): Promise<void> {
  await callCommand("window_set_hit_region", {
    x: region.x,
    y: region.y,
    width: region.width,
    height: region.height,
  });
}

/**
 * 穿透总闸（D1 输入框可见 / D4 边缘休眠时传 false → 整窗接收点击）。
 * 默认 true：自动穿透状态机由 Rust 驱动。
 */
export async function setClickThroughEnabled(enabled: boolean): Promise<void> {
  await callCommand("window_set_click_through_enabled", { enabled });
}

/**
 * 气泡防截断布局（D1.1，纯函数、可单测）。
 *
 * 决策：猫右缘空间不足（含 SAFETY 安全边距）则翻到左侧；带**滞回**（切换阈值差
 * ≥ HYSTERESIS）防止边缘抖动。返回气泡落点与该侧最大可用宽度（渲染层据此 clamp）。
 *
 * 纪律：纯函数——不碰 DOM / 响应式，全部参数注入（DoD 单测覆盖三分支）。
 */

import type { HitRegion } from "../api/types";

/** 安全边距（px）：气泡与猫身/窗口边缘至少留白。 */
export const SAFETY_MARGIN = 8;
/** 翻转滞回（px）：当前侧剩余空间低于对侧超过此值才翻，防抖动。 */
export const FLIP_HYSTERESIS = 24;

export interface BubbleLayoutInput {
  /** 窗口 A 逻辑宽度（px）。 */
  winW: number;
  /** 猫身命中区域（窗口逻辑像素）。 */
  catRegion: HitRegion;
  /** 气泡期望宽度（px；渲染层用于判断哪侧放得下）。 */
  bubbleW: number;
  /** 上一次的 side（滞回依据；首次给 undefined）。 */
  prevSide?: "left" | "right";
}

export interface BubbleLayout {
  side: "left" | "right";
  /** 气泡左偏移（px，已 clamp 在窗口内）。 */
  x: number;
  /** 该侧可用最大宽度（px，渲染层 clamp 用，防溢出窗口）。 */
  maxWidth: number;
}

/** 选择气泡侧（右侧优先，带滞回；两侧都不够则选较宽侧）。 */
export function chooseSide(
  winW: number,
  region: HitRegion,
  bubbleW: number,
  prevSide?: "left" | "right",
): "left" | "right" {
  const catRight = region.x + region.width;
  const rightSpace = winW - catRight;
  const leftSpace = region.x;

  const rightFits = rightSpace - bubbleW >= SAFETY_MARGIN;
  const leftFits = leftSpace - bubbleW >= SAFETY_MARGIN;

  if (prevSide) {
    // 滞回：当前侧仍"够用（留有余量）"则保持，避免抖动
    if (prevSide === "right" && rightSpace - bubbleW >= SAFETY_MARGIN + FLIP_HYSTERESIS) {
      return "right";
    }
    if (prevSide === "left" && leftSpace - bubbleW >= SAFETY_MARGIN + FLIP_HYSTERESIS) {
      return "left";
    }
  }
  if (rightFits) return "right";
  if (leftFits) return "left";
  // 都不够：选较宽侧（仍可能溢出，但至少选空间大的一边）
  return rightSpace >= leftSpace ? "right" : "left";
}

/** 解析气泡布局（落点 + 最大宽度，均已 clamp 在窗口内）。 */
export function resolveBubbleLayout(input: BubbleLayoutInput): BubbleLayout {
  const { winW, catRegion, bubbleW, prevSide } = input;
  const side = chooseSide(winW, catRegion, bubbleW, prevSide);
  const catRight = catRegion.x + catRegion.width;

  let x: number;
  let maxWidth: number;
  if (side === "right") {
    x = catRight + SAFETY_MARGIN;
    maxWidth = Math.max(SAFETY_MARGIN, winW - x - SAFETY_MARGIN);
  } else {
    maxWidth = Math.max(SAFETY_MARGIN, catRegion.x - SAFETY_MARGIN);
    x = catRegion.x - SAFETY_MARGIN - bubbleW;
  }
  // clamp 在窗口内（防止左侧翻折后超出左缘）
  x = Math.max(SAFETY_MARGIN, Math.min(x, winW - SAFETY_MARGIN));
  return { side, x, maxWidth };
}

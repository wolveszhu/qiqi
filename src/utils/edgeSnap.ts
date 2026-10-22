/**
 * 边缘吸附判定（D4.1 纯函数，Vitest 覆盖）：拖拽松手位置 → 是否贴边。
 *
 * 坐标系：pos 与 monitor 均为**逻辑像素**（Rust `window_get_outer_position` 已统一转换，
 * 规避多屏 DPI 坑，详见 §五 风险登记）。monitor = 当前显示器矩形。
 *
 * 双角优先级：top > left/right（上缘吸附最自然，避免左上角歧义）。
 * 阈值常量集中（勘误 #15：多屏误差放大至 32，待 F3 实测调参）。
 */

export type SnapEdge = "top" | "left" | "right";

export interface ScreenRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

/** 吸附阈值（px）。多屏 DPI 误差容忍。 */
export const SNAP_THRESHOLD = 32;

/**
 * 判定 pos 是否贴边。返回 edge 或 null。
 * - top：pos.y 距显示器上缘 ≤ threshold
 * - left：pos.x 距左缘 ≤ threshold（top 不命中时）
 * - right：pos.x 距右缘 ≤ threshold（top 不命中时）
 */
export function detectSnapEdge(
  pos: Point,
  monitor: ScreenRect,
  threshold: number = SNAP_THRESHOLD,
): SnapEdge | null {
  const topHit = pos.y <= monitor.y + threshold;
  if (topHit) return "top";
  const leftHit = pos.x <= monitor.x + threshold;
  if (leftHit) return "left";
  const rightHit = pos.x >= monitor.x + monitor.width - threshold;
  if (rightHit) return "right";
  return null;
}

/** 当前是否处于吸附态（贴边判定复用；供 restore 参考，可选）。 */
export function isSnapped(
  pos: Point,
  monitor: ScreenRect,
  threshold: number = SNAP_THRESHOLD,
): boolean {
  return detectSnapEdge(pos, monitor, threshold) !== null;
}

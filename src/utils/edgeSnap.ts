/**
 * 边缘吸附判定（D4.1 纯函数，Vitest 覆盖）：拖拽松手位置 → 是否贴边。
 *
 * 坐标系：pos 与 monitor 均为**逻辑像素**（Rust `window_get_outer_position` 已统一转换，
 * 规避多屏 DPI 坑，详见 §五 风险登记）。monitor = 当前显示器矩形。
 *
 * **仅左右两侧吸附**（用户决策：上下不吸附）。
 * right 判定必须用**窗口右缘**（x + width）——旧实现拿窗口左缘 x 与右缘比对，
 * 阈值 32px < 窗口宽 150px，右侧永不命中（回归缺陷）。
 * 阈值常量集中（勘误 #15：多屏误差放大至 32，待 F3 实测调参）。
 */

export type SnapEdge = "left" | "right";

export interface ScreenRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** 窗口外位置 + 外尺寸（逻辑 px；Rust outer_position_view 下发）。 */
export interface SnapPosition {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** 吸附阈值（px）。多屏 DPI 误差容忍。 */
export const SNAP_THRESHOLD = 32;

/**
 * 吸附态露出比例（占形象"圆盘"直径的比例）：默认 / 下界 / 上界。
 *
 * 0.5 = 露出半个圆（默认）；越大露出越多（如 0.7 连鼻子一起露出）。
 * 前端滑块与后端 clamp 共用这组常量，避免两端不一致。
 */
export const SNAP_SHOW_RATIO_DEFAULT = 0.5;
export const SNAP_SHOW_RATIO_MIN = 0.3;
export const SNAP_SHOW_RATIO_MAX = 0.8;

/**
 * 判定 pos 是否贴左右边缘。返回 edge 或 null。
 * - left：窗口左缘 pos.x 距显示器左缘 ≤ threshold
 * - right：窗口右缘 pos.x + width 距显示器右缘 ≤ threshold
 */
export function detectSnapEdge(
  pos: SnapPosition,
  monitor: ScreenRect,
  threshold: number = SNAP_THRESHOLD,
): SnapEdge | null {
  const leftHit = pos.x <= monitor.x + threshold;
  if (leftHit) return "left";
  const rightHit = pos.x + pos.width >= monitor.x + monitor.width - threshold;
  if (rightHit) return "right";
  return null;
}

/** 当前是否处于吸附态（贴边判定复用；供 restore 参考，可选）。 */
export function isSnapped(
  pos: SnapPosition,
  monitor: ScreenRect,
  threshold: number = SNAP_THRESHOLD,
): boolean {
  return detectSnapEdge(pos, monitor, threshold) !== null;
}

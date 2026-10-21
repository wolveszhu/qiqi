/**
 * 果冻物理纯函数集（桌宠.md §4.2 / 设计基线 §D 公式）
 *
 * 约束（code-standards §6 / 目录规范 §3）：
 * - 零 I/O、零 PIXI 依赖、帧内零分配友好——全部为无副作用纯函数，可 Vitest 单测
 * - 渲染器（CodeCatRenderer）在 Ticker 回调中调用这些函数，仅回写 transform
 */

/**
 * 呼吸缩放：正弦驱动的横向压扁 / 纵向拉伸（baseline §D 呼吸公式）。
 *
 * @param t - 相位（秒或 Ticker deltaMS 累积，由调用方决定周期）
 * @param amplitude - 呼吸幅值（默认 0.03，即纵向 ±3%）
 * @returns [scaleX, scaleY] —— 横向压扁为纵向拉伸的一半，保持体积感
 *
 * @example breathScales(0) // [1, 1] 相位起点无扰动
 */
export function breathScales(t: number, amplitude = 0.03): [number, number] {
  const s = Math.sin(t) * amplitude;
  return [1 - s / 2, 1 + s];
}

/**
 * 阻尼回归：形变值向基准 1 指数收敛（baseline §D 果冻阻尼系数 0.1）。
 * 高于 1 时向下回弹，低于 1 时向上回弹，单调不越过基准。
 *
 * @param current - 当前形变值（如拉伸后的 scaleY）
 * @param factor - 阻尼系数（默认 0.1，即每帧收敛剩余差距的 10%）
 */
export function dampedRecovery(current: number, factor = 0.1): number {
  return current + (1 - current) * factor;
}

/**
 * 拖拽拉伸因子：按位移线性放大，上限 1.5 封顶（baseline §D Stretch&Squash）。
 *
 * @param dx - 指针位移 X
 * @param dy - 指针位移 Y
 * @param scale - 像素→形变换算（默认 0.01，即 50px → 0.5）
 * @param max - 封顶（默认 1.5）
 */
export function stretchFactor(dx: number, dy: number, scale = 0.01, max = 1.5): number {
  return Math.min(Math.hypot(dx, dy) * scale, max);
}

/**
 * 视线跟随偏移：瞳孔朝目标方向偏移，限幅 max（baseline §D 视线跟随 3-5px）。
 * 距离为零时返回 {0,0}（除零保护），距离超过 max 时按方向单位化到 max。
 *
 * @returns {dx, dy} 相对瞳孔基准位的偏移量
 */
export function eyeOffset(
  px: number,
  py: number,
  tx: number,
  ty: number,
  max = 5,
): { dx: number; dy: number } {
  const dx = tx - px;
  const dy = ty - py;
  const dist = Math.hypot(dx, dy);
  if (dist === 0) {
    return { dx: 0, dy: 0 };
  }
  const k = Math.min(dist, max) / dist;
  return { dx: dx * k, dy: dy * k };
}

/**
 * 果冻冲击：点击/落地瞬间的一次性形变（拉宽压扁，baseline §D 1.15/0.85）。
 * 冲击后由 dampedRecovery 逐帧回归基准。
 */
export function jellyImpulse(): [number, number] {
  return [1.15, 0.85];
}

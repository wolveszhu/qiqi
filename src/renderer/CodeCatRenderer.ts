/**
 * 代码猫渲染器（骨架占位）：C1 任务落位完整实现。
 *
 * TODO(C1):
 * - PIXI.Graphics 分层绘制：bodyG / faceG / ears / tailG(Bezier, 旋转而非重绘) / eyesG；
 * - 单一 Ticker：正弦呼吸 + 阻尼果 jelly 形变 + 拖拽 Stretch&Squash + lookAt（physics.ts 纯函数）；
 * - 绘制一次、每帧仅改 transform、帧内零 new（code-standards §6）；
 * - getHitRegion 返回身体包围盒（union 规则由 PetWindow 扩展，D4 铃铛）。
 */

import type { HitRegion, IPetRenderer, PetAction, PetExpression } from "./IPetRenderer";

export class CodeCatRenderer implements IPetRenderer {
  private container: HTMLElement | null = null;
  /** 空闲降帧标记（E2 读它决定 maxFPS；C1/E2 接线）。 */
  idleMode = false;

  async mount(container: HTMLElement): Promise<void> {
    this.container = container;
    // TODO(C1): PIXI Application.init + 分层 Graphics + Ticker.add
  }

  playAction(name: PetAction): void {
    // TODO(C1): 按动作触发一次性形变（jellyImpulse → dampedRecovery 回归）
    void name;
  }

  setExpression(name: PetExpression): void {
    // TODO(C1): 切换眼睛/嘴形容器
    void name;
  }

  lookAt(x: number, y: number): void {
    // TODO(C1): eyeOffset(px, py, x, y) → 瞳孔 transform
    void x;
    void y;
  }

  getHitRegion(): HitRegion {
    // TODO(C1): 返回 bodyG 世界包围盒；骨架期返回容器占位
    const rect = this.container?.getBoundingClientRect();
    return { x: 0, y: 0, width: rect?.width ?? 400, height: rect?.height ?? 400 };
  }

  setIdleMode(enabled: boolean): void {
    this.idleMode = enabled;
    // TODO(E2): app.ticker.maxFPS = enabled ? 30 : 60
  }

  dispose(): void {
    // TODO(C1): app.destroy(true, { children: true }) + 移除全部监听
    this.container = null;
  }
}

/**
 * 自定义精灵渲染器（Phase 3 自定义形象）。
 *
 * 与内置猫狗（PetRendererBase 矢量绘制）的区别：
 * - 用 PIXI `Sprite` 加载用户导入的 PNG，而非 `Graphics` 矢量分层；
 * - 复用 `physics.ts` 纯函数做**整体变换生命感**：呼吸缩放 / 点击果冻 / 边缘吸附躺倒；
 * - **不含**眨眼 / 视线跟随 / 摆尾（这些依赖矢量分层，单图无此结构，基线 §D 明确区分）。
 *
 * 几何模型（锚点 = 边缘吸附旋转中心）：
 * - 精灵 `anchor` 设为 `cfg.anchor`，使「圆盘中心」落在 body 原点；
 * - body 原点对齐 petRoot 原点（即舞台中心），故锚点 = 舞台中心 = 旋转中心；
 * - 边缘吸附时整体绕锚点旋转，躺倒露出用户期望的部位（微调 UI 即调此锚点）。
 *
 * 纪律（规范 §6 / §I）：单个 Ticker；帧内零 `new`；PIXI 实例由普通变量持有，绝不进响应式。
 */

import { Application, Assets, Container, Sprite, type Texture } from "pixi.js";

import type { IPetRenderer } from "./IPetRenderer";
import type { CustomAvatarConfig } from "./customAvatarRegistry";
import { SNAP_SHOW_RATIO_DEFAULT, type SnapEdge } from "../utils/edgeSnap";
import { breathScales, dampedRecovery, jellyImpulse } from "./physics";

export class CustomSpriteRenderer implements IPetRenderer {
  /** 固定配置（导入时由注册表注入；滑杆微调走 in-memory 更新后重建）。 */
  private cfg: CustomAvatarConfig;
  private app?: Application;
  private body?: Container;

  /** 图片自然尺寸（纹理加载后填充）。 */
  private naturalW = 1;
  private naturalH = 1;

  /** 舞台像素尺寸（resizeTo 容器，mount 时记录）。 */
  private screenW = 400;
  private screenH = 400;

  /** 物理状态（普通变量，非响应式）。 */
  private phi = 0;
  private jelly: [number, number] = [1, 1];
  private sleepAngle = 0;
  private sleepTarget = 0;
  private lastSide: SnapEdge | null = null;
  private lastRatio = SNAP_SHOW_RATIO_DEFAULT;
  private idleMode = false;

  constructor(cfg: CustomAvatarConfig) {
    this.cfg = cfg;
  }

  async mount(container: HTMLElement): Promise<void> {
    const app = new Application();
    await app.init({
      background: "transparent",
      backgroundAlpha: 0,
      antialias: true,
      resizeTo: container,
      autoDensity: true,
      resolution: window.devicePixelRatio || 1,
    });
    container.appendChild(app.canvas);
    this.app = app;

    this.screenW = app.screen.width;
    this.screenH = app.screen.height;

    const petRoot = new Container();
    petRoot.position.set(this.screenW / 2, this.screenH / 2);
    app.stage.addChild(petRoot);

    // body 原点对齐 petRoot 原点 → 锚点（圆盘中心）即舞台中心、即旋转中心
    const body = new Container();
    body.position.set(0, 0);
    petRoot.addChild(body);
    this.body = body;

    const texture = await Assets.load<Texture>(this.cfg.dataUrl);
    this.naturalW = texture.width;
    this.naturalH = texture.height;

    const sprite = new Sprite(texture);
    sprite.anchor.set(this.cfg.anchorX, this.cfg.anchorY); // 锚点 = 圆盘中心
    sprite.position.set(0, 0); // 锚点落在 body 原点
    sprite.scale.set(this.cfg.scale); // 用户整体缩放（固定）
    body.addChild(sprite);

    app.ticker.add(this.updateTransforms);
    if (this.idleMode) app.ticker.maxFPS = 30;
  }

  /**
   * 单一 Ticker：呼吸 + 果冻衰减 + 边缘吸附旋转（复用 physics 纯函数）。
   * 帧内零分配：临时量（[bx, by] 为短数组）属规整返回，base 同款，可接收。
   */
  private updateTransforms = (): void => {
    if (!this.body || !this.app) return;
    const dt = this.app.ticker.deltaMS;
    this.phi += (dt / 1000) * 2; // breathRate = 2

    this.jelly[0] = dampedRecovery(this.jelly[0]);
    this.jelly[1] = dampedRecovery(this.jelly[1]);

    const [bx, by] = breathScales(this.phi);
    const sx = bx * this.jelly[0];
    const sy = by * this.jelly[1];
    this.body.scale.set(sx, sy);

    this.sleepAngle += (this.sleepTarget - this.sleepAngle) * 0.12;
    if (Math.abs(this.sleepTarget - this.sleepAngle) < 0.001) {
      this.sleepAngle = this.sleepTarget;
    }
    this.body.rotation = this.sleepAngle;
  };

  playAction(name: string): void {
    if (name === "click") {
      this.jelly = jellyImpulse(); // 整体 Q 弹（拉宽压扁）
    } else if (name === "sleep") {
      this.setEdgeSleep(true, this.lastSide, this.lastRatio);
    } else if (name === "stretch" || name === "wake") {
      this.setEdgeSleep(false);
    }
    // 'idle' 等无额外动作
  }

  /**
   * 自定义形象为单图，无矢量表情分层：表情以「整体缩放微调」近似（开心略微膨胀），
   * 与呼吸/果冻同走 body.scale；保持接口契约一致性，不做视线/眨眼。neutral 即复位。
   */
  setExpression(name: string): void {
    if (name === "happy") this.body?.scale.set(1.04, 1.04);
    else if (name === "angry") this.body?.scale.set(0.96, 0.96);
    // neutral / 其它：维持呼吸驱动（updateTransforms 每帧重写 body.scale）
  }

  /** 无矢量眼睛，视线跟随无作用（接口保留）。 */
  lookAt(_x: number, _y: number): void {
    /* no-op for sprite avatar */
  }

  setEdgeSleep(
    sleeping: boolean,
    side: SnapEdge | null = null,
    showRatio = SNAP_SHOW_RATIO_DEFAULT,
  ): void {
    this.lastSide = side;
    this.lastRatio = showRatio;
    // 左=顺时针(-π)、右=逆时针(+π)；showRatio 决定躺倒幅度（D4.4 露出比例）
    this.sleepTarget = sleeping ? (side === "left" ? -1 : 1) * Math.PI * showRatio : 0;
  }

  setIdleMode(enabled: boolean): void {
    this.idleMode = enabled;
    if (this.app) this.app.ticker.maxFPS = enabled ? 30 : 60;
  }

  getHitRegion(): { x: number; y: number; width: number; height: number } {
    if (!this.app) {
      return { x: 0, y: 0, width: this.screenW, height: this.screenH };
    }
    const s = this.cfg.scale;
    const w = this.naturalW * s;
    const h = this.naturalH * s;
    // sprite 顶左在 petRoot 局部坐标 = (-anchorX*W, -anchorY*H)，petRoot 居中
    const cx = this.app.screen.width / 2;
    const cy = this.app.screen.height / 2;
    return {
      x: cx - this.cfg.anchorX * w,
      y: cy - this.cfg.anchorY * h,
      width: w,
      height: h,
    };
  }

  dispose(): void {
    if (this.app) {
      this.app.ticker.remove(this.updateTransforms);
      this.app.destroy(true, { children: true });
      this.app = undefined;
      this.body = undefined;
    }
  }
}

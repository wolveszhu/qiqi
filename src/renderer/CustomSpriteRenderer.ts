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

import type { HitRegion, IPetRenderer, PetAction } from "./IPetRenderer";
import type { CustomAvatarConfig } from "./customAvatarRegistry";
import { SNAP_SHOW_RATIO_DEFAULT, type SnapEdge } from "../utils/edgeSnap";
import { ensureValidHitRegion } from "./PetRendererBase";
import { breathScales, dampedRecovery, jellyImpulse, rotatedHitRegion } from "./physics";

export class CustomSpriteRenderer implements IPetRenderer {
  /** 自适应基准：scale=1 时图片最长边占舞台短边的比例（与内置形象观感协调）。 */
  private static readonly STAGE_FIT_RATIO = 0.5;

  /** 固定配置（导入时由注册表注入；滑杆微调走 in-memory 更新后重建）。 */
  private cfg: CustomAvatarConfig;
  private app?: Application;
  /** 舞台根容器（居中锚定；窗口尺寸变化时更新 position，见 relayout）。 */
  private root?: Container;
  private body?: Container;

  /** 图片自然尺寸（纹理加载后填充）。 */
  private naturalW = 1;
  private naturalH = 1;

  /** 自适应基准缩放（mount 时按图片与舞台尺寸算出；`cfg.scale` 是其上的倍率）。 */
  private fitScale = 1;

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

  /** 命中矩形缓存（复用同一对象，500ms 上报一次；与 PetRendererBase 同款纪律）。 */
  private readonly hitRegion: HitRegion = { x: 0, y: 0, width: 0, height: 0 };

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
      // 与基类同款 2 上限：300% DPI 屏裸用 devicePixelRatio 会让纹理分辨率爆炸
      resolution: Math.min(window.devicePixelRatio || 1, 2),
    });
    container.appendChild(app.canvas);
    this.app = app;

    this.screenW = app.screen.width;
    this.screenH = app.screen.height;

    const petRoot = new Container();
    petRoot.position.set(this.screenW / 2, this.screenH / 2);
    app.stage.addChild(petRoot);
    this.root = petRoot;

    // body 原点对齐 petRoot 原点 → 锚点（圆盘中心）即舞台中心、即旋转中心
    const body = new Container();
    body.position.set(0, 0);
    petRoot.addChild(body);
    this.body = body;

    const texture = await Assets.load<Texture>(this.cfg.dataUrl);
    this.naturalW = texture.width;
    this.naturalH = texture.height;

    // 用户图片尺寸不可控（几百到几千像素）：按「最长边 = 舞台短边 × STAGE_FIT_RATIO」
    // 自适应归一，cfg.scale 作为倍率微调。旧实现把 cfg.scale 直接当精灵缩放，
    // scale=1 会按原始像素绘制，大图直接放大到只露出局部（耳朵/鼻子的裁切特写）。
    this.fitScale =
      (Math.min(this.screenW, this.screenH) * CustomSpriteRenderer.STAGE_FIT_RATIO) /
      Math.max(this.naturalW, this.naturalH);

    const sprite = new Sprite(texture);
    sprite.anchor.set(this.cfg.anchorX, this.cfg.anchorY); // 锚点 = 圆盘中心
    sprite.position.set(0, 0); // 锚点落在 body 原点
    sprite.scale.set(this.fitScale * this.cfg.scale); // 自适应 × 用户倍率
    body.addChild(sprite);

    app.ticker.add(this.updateTransforms);
    if (this.idleMode) app.ticker.maxFPS = 30;
    // 窗口展开/收起 → 重新居中：petRoot 位置在 mount 时固定，不同步会在窗口
    // 尺寸变化后与 getHitRegion（每次现读 app.screen）错开，点击判定错位。
    window.addEventListener("resize", this.relayout);
  }

  /**
   * 窗口尺寸变化 → 同步 resize 后重新居中（同 PetRendererBase.relayout：
   * PIXI `resizeTo` 经 ResizeObserver → ticker 异步生效，显式 `app.resize()`
   * 防止命中区按旧 `app.screen` 上报）。
   */
  private readonly relayout = (): void => {
    requestAnimationFrame(() => {
      const app = this.app;
      const root = this.root;
      if (!app || !root) return;
      app.resize();
      this.screenW = app.screen.width;
      this.screenH = app.screen.height;
      root.position.set(this.screenW / 2, this.screenH / 2);
    });
  };

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

  playAction(name: PetAction): void {
    switch (name) {
      case "click":
        this.jelly = jellyImpulse(); // 整体 Q 弹（拉宽压扁）
        break;
      case "sleep":
        this.setEdgeSleep(true, this.lastSide, this.lastRatio);
        break;
      case "stretch":
        this.setEdgeSleep(false);
        break;
      default:
        break; // idle / drag / eat：单图无矢量分层，无对应动作
    }
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

  /**
   * 命中区：整图包围盒（含透明边）绕锚点旋转后的窗口 AABB。
   *
   * 休眠躺倒是任意角度（±π·showRatio 渐变），必须按当前 `sleepAngle` 实时求
   * 旋转 AABB 并裁切窗口——旧实现恒按未旋转矩形返回，躺倒后命中区与视觉错位，
   * 点击判定落到形象外的直立矩形上。裁切退化为 0 → 整窗回退（穿透死锁防线，
   * 与基类 `ensureValidHitRegion` 同向兜底）。呼吸/果冻缩放（±3%/1.15）不反映，
   * 与基类口径一致。
   */
  getHitRegion(): HitRegion {
    // 与 mount 的精灵缩放同源：自适应基准 × 用户倍率（否则命中区与视觉错位）
    const s = this.fitScale * this.cfg.scale;
    const w = this.naturalW * s;
    const h = this.naturalH * s;
    const app = this.app;
    if (!app) {
      return this.fillRegion(0, 0, this.screenW, this.screenH);
    }
    // sprite 顶左在 petRoot 局部坐标 = (-anchorX·W, -anchorY·H)；petRoot 居中 = 旋转中心
    const region = rotatedHitRegion(
      app.screen.width / 2,
      app.screen.height / 2,
      w,
      h,
      -this.cfg.anchorX * w,
      -this.cfg.anchorY * h,
      this.sleepAngle,
      this.screenW,
      this.screenH,
    );
    const valid = ensureValidHitRegion(
      region.x,
      region.y,
      region.width,
      region.height,
      this.screenW,
      this.screenH,
    );
    return this.fillRegion(valid.x, valid.y, valid.width, valid.height);
  }

  /** 逐字段写回复用对象（外部可能已持有引用，见 PetRendererBase.updateHitRegion）。 */
  private fillRegion(x: number, y: number, width: number, height: number): HitRegion {
    this.hitRegion.x = x;
    this.hitRegion.y = y;
    this.hitRegion.width = width;
    this.hitRegion.height = height;
    return this.hitRegion;
  }

  dispose(): void {
    window.removeEventListener("resize", this.relayout);
    if (this.app) {
      this.app.ticker.remove(this.updateTransforms);
      this.app.destroy(true, { children: true });
      this.app = undefined;
      this.root = undefined;
      this.body = undefined;
    }
  }
}

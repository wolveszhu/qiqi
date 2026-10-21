/**
 * 代码猫渲染器（C1 完整实现）：PIXI.Graphics 绘制 + 单一 Ticker 动画 + 交互状态机。
 *
 * 性能红线（code-standards §6 / 规范 §3）：
 * - 所有图形**绘制一次**；每帧只改 transform（scale/rotation/position）；
 * - Ticker 回调体内**零 new、零对象/数组字面量**（复用类字段）；
 * - PIXI 实例由普通类字段持有，**绝不进 Vue 响应式**；
 * - 卸载走 `destroy(true)` 并移除全部监听。
 *
 * 契约：`IPetRenderer`（baseline §D）；Phase 3 替换点为 `factory.ts` 注册表。
 */

import { Application, Container, Graphics } from "pixi.js";

import type { Ticker } from "pixi.js";
import { breathScales, dampedRecovery, eyeOffset, jellyImpulse } from "./physics";
import type { HitRegion, IPetRenderer, PetAction, PetExpression } from "./IPetRenderer";

/** 画布逻辑尺寸（与窗口 A 的 400×400 一致，B1 已配置）。 */
const CANVAS_SIZE = 400;

/** 配色：与 styles/tokens.css 的 Jelly-Toy 设计 token 对齐。 */
const COLOR_BODY = 0x7ca65b;
const COLOR_BODY_DEEP = 0x5d8541;
const COLOR_FACE = 0xfffef9;
const COLOR_INK = 0x403a2e;
const COLOR_BLUSH = 0xf5aeb9;
const COLOR_NOSE = 0xff9e5e;

/** 动画常量（集中定义，防散落魔法数）。 */
const BREATH_OMEGA = 0.0025; // rad/ms → 周期约 2.5s
const TAIL_OMEGA = 0.0021; // 尾巴摆动角频率
const TAIL_AMPLITUDE = 0.18; // 弧度
const BLINK_DURATION_MS = 140;
const BLINK_MIN_GAP_MS = 2600;
const BLINK_GAP_RANGE_MS = 2600;
const EXPRESSION_HOLD_MS = 700; // click 触发 happy 的持续时间

export class CodeCatRenderer implements IPetRenderer {
  // ---- PIXI 实例（普通字段，非响应式） ----
  private app: Application | null = null;
  private petRoot: Container | null = null;
  private eyesG: Container | null = null;
  private tailG: Container | null = null;

  // ---- 动画状态（循环外复用，帧内零分配） ----
  private elapsedMs = 0;
  private scaleX = 1;
  private scaleY = 1;
  private lookX = 0;
  private lookY = 0;
  private eyesBaseX = 0;
  private eyesBaseY = 0;
  private centerX = CANVAS_SIZE / 2;
  private centerY = CANVAS_SIZE / 2 - 6;
  private blinkTimer = BLINK_MIN_GAP_MS;
  private blinkElapsed = 0;
  private blinking = false;
  private expression: PetExpression = "neutral";
  private expressionHoldMs = 0;
  /** 空闲降帧标记（公开以免 TS 未读告警；E2 通过 setIdleMode 驱动 maxFPS）。 */
  idleMode = false;

  /** 包围盒缓存对象（复用同一对象，避免每帧分配）。 */
  private readonly hitRegion: HitRegion = { x: 0, y: 0, width: 0, height: 0 };

  /** 单一 Ticker 回调：呼吸 + 果冻回弹 + 尾巴 + 眨眼 + 视线跟随。 */
  private readonly tick = (ticker: Ticker): void => {
    const dt = ticker.deltaMS;
    this.elapsedMs += dt;

    const root = this.petRoot;
    const tail = this.tailG;
    const eyes = this.eyesG;
    if (!root || !tail || !eyes) return;

    // 1) 呼吸（sleep 态幅度减半）
    const amp = this.expression === "sleepy" ? 0.015 : 0.03;
    const [bx, by] = breathScales(this.elapsedMs * BREATH_OMEGA, amp);

    // 2) 果冻冲击后的阻尼回弹（与呼吸相乘叠加，非覆盖）
    this.scaleX = dampedRecovery(this.scaleX);
    this.scaleY = dampedRecovery(this.scaleY);
    root.scale.set(this.scaleX * bx, this.scaleY * by);

    // 3) 尾巴摆动（旋转近似，不重绘几何）
    tail.rotation = Math.sin(this.elapsedMs * TAIL_OMEGA) * TAIL_AMPLITUDE;

    // 4) 眨眼（sleep 态保持闭眼，不走眨眼计时）
    if (this.expression === "sleepy") {
      eyes.scale.set(1, 0.08);
    } else {
      this.updateBlink(dt, eyes);
    }

    // 5) 视线跟随（限幅 5px，作用于眼睛容器）
    const offset = eyeOffset(this.centerX, this.centerY, this.lookX, this.lookY, 5);
    eyes.position.set(this.eyesBaseX + offset.dx, this.eyesBaseY + offset.dy);

    // 6) 临时表情（click 触发的 happy）到期复位
    if (this.expressionHoldMs > 0) {
      this.expressionHoldMs -= dt;
      if (this.expressionHoldMs <= 0) this.setExpression("neutral");
    }
  };

  /** 开发期视线数据源：window 级 mousemove（B5 probe 事件接入后一行替换）。 */
  private readonly onMouseMove = (ev: MouseEvent): void => {
    this.lookAt(ev.clientX, ev.clientY);
  };

  async mount(container: HTMLElement): Promise<void> {
    const app = new Application();
    await app.init({
      backgroundAlpha: 0, // 窗口 A 透明（B1）
      antialias: true,
      resizeTo: container,
      autoDensity: true,
      resolution: Math.min(window.devicePixelRatio || 1, 2),
    });
    container.appendChild(app.canvas);
    this.app = app;

    // ---- 分层绘制（仅一次） ----
    const petRoot = new Container();
    petRoot.position.set(this.centerX, this.centerY);
    this.petRoot = petRoot;
    app.stage.addChild(petRoot);

    // 尾巴（独立容器：pivot 在根部，每帧仅 rotation）
    const tailG = new Container();
    tailG.position.set(58, 30);
    tailG.pivot.set(58, 30);
    const tail = new Graphics()
      .moveTo(58, 30)
      .bezierCurveTo(104, 6, 126, 58, 104, 96)
      .stroke({ width: 14, color: COLOR_BODY_DEEP, cap: "round" });
    tailG.addChild(tail);
    this.tailG = tailG;

    // 身体 + 面部 + 耳朵 + 腮红 + 鼻子（静态，绘制一次）
    const bodyG = new Graphics()
      .roundRect(-68, -56, 136, 114, 46)
      .fill(COLOR_BODY)
      .stroke({ width: 3, color: COLOR_BODY_DEEP, alpha: 0.85 })
      .ellipse(0, -8, 54, 46)
      .fill(COLOR_FACE)
      .ellipse(-34, 14, 11, 7)
      .fill({ color: COLOR_BLUSH, alpha: 0.55 })
      .ellipse(34, 14, 11, 7)
      .fill({ color: COLOR_BLUSH, alpha: 0.55 })
      .ellipse(0, 6, 6, 4)
      .fill(COLOR_NOSE)
      .ellipse(0, -44, 32, 15)
      .fill(COLOR_BODY)
      .poly([-60, -50, -36, -96, -18, -56])
      .fill(COLOR_BODY)
      .stroke({ width: 2, color: COLOR_BODY_DEEP, alpha: 0.8 })
      .poly([60, -50, 36, -96, 18, -56])
      .fill(COLOR_BODY)
      .stroke({ width: 2, color: COLOR_BODY_DEEP, alpha: 0.8 });

    // 眼睛容器（lookAt / 眨眼 / 表情作用于此）
    const eyesG = new Container();
    this.eyesBaseX = 0;
    this.eyesBaseY = -14;
    eyesG.position.set(this.eyesBaseX, this.eyesBaseY);
    const eyeL = new Graphics().roundRect(-36, -13, 20, 26, 10).fill(COLOR_INK);
    const eyeR = new Graphics().roundRect(16, -13, 20, 26, 10).fill(COLOR_INK);
    eyesG.addChild(eyeL, eyeR);
    this.eyesG = eyesG;

    petRoot.addChild(tailG, bodyG, eyesG);

    // ---- 包围盒：挂载时计算一次（帧内不再 getBounds） ----
    const rect = petRoot.getBounds().rectangle;
    this.hitRegion.x = rect.x;
    this.hitRegion.y = rect.y;
    this.hitRegion.width = rect.width;
    this.hitRegion.height = rect.height;

    // ---- 单一 Ticker + 开发期数据源 ----
    app.ticker.add(this.tick, this);
    window.addEventListener("mousemove", this.onMouseMove);
  }

  playAction(name: PetAction): void {
    switch (name) {
      case "click": {
        // 果冻冲击（拉宽压扁）+ 短暂 happy 表情
        const [sx, sy] = jellyImpulse();
        this.scaleX = sx;
        this.scaleY = sy;
        this.setExpression("happy");
        this.expressionHoldMs = EXPRESSION_HOLD_MS;
        break;
      }
      case "drag":
        // C2 接入后按位移驱动 stretchFactor 形变；当前给一次轻压缩反馈
        this.scaleX = 1.08;
        this.scaleY = 0.94;
        break;
      case "sleep":
        this.setExpression("sleepy");
        break;
      case "stretch":
        this.scaleX = 0.82;
        this.scaleY = 1.22;
        this.expressionHoldMs = 0;
        break;
      case "idle":
        this.expressionHoldMs = 0;
        this.setExpression("neutral");
        this.scaleX = 1;
        this.scaleY = 1;
        break;
    }
  }

  setExpression(name: PetExpression): void {
    this.expression = name;
    const eyes = this.eyesG;
    if (!eyes) return;
    switch (name) {
      case "neutral":
        eyes.scale.set(1, 1);
        break;
      case "happy":
        eyes.scale.set(1, 0.55); // 弯眼近似（baseline §D）
        break;
      case "angry":
        eyes.scale.set(1.1, 0.8);
        break;
      case "sleepy":
        eyes.scale.set(1, 0.08); // 闭合横线
        break;
    }
  }

  lookAt(x: number, y: number): void {
    this.lookX = x;
    this.lookY = y;
  }

  getHitRegion(): HitRegion {
    // 复用同一对象（C2 每 100ms 读取上报；返回值不应被外部长期持有修改）
    return this.hitRegion;
  }

  setIdleMode(enabled: boolean): void {
    this.idleMode = enabled;
    if (this.app) this.app.ticker.maxFPS = enabled ? 30 : 60;
  }

  dispose(): void {
    window.removeEventListener("mousemove", this.onMouseMove);
    const app = this.app;
    this.app = null;
    this.petRoot = null;
    this.eyesG = null;
    this.tailG = null;
    app?.destroy(true, { children: true });
  }

  /** 眨眼状态推进（sleep 态不调用）。 */
  private updateBlink(dt: number, eyes: Container): void {
    if (this.blinking) {
      this.blinkElapsed += dt;
      const t = this.blinkElapsed / BLINK_DURATION_MS;
      // 1 → 0 → 1 的快速闭合
      const sy = t < 0.5 ? 1 - t * 2 : (t - 0.5) * 2;
      eyes.scale.set(1, Math.max(0.08, sy));
      if (this.blinkElapsed >= BLINK_DURATION_MS) {
        this.blinking = false;
        this.blinkElapsed = 0;
        eyes.scale.set(1, 1);
        this.blinkTimer = BLINK_MIN_GAP_MS + Math.random() * BLINK_GAP_RANGE_MS;
      }
      return;
    }
    this.blinkTimer -= dt;
    if (this.blinkTimer <= 0) {
      this.blinking = true;
      this.blinkElapsed = 0;
    }
  }
}

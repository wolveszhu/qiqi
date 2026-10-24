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

/** 画布逻辑尺寸初始默认值（mount 时按容器实际尺寸重算）。 */
const CANVAS_SIZE = 240;

/** 形象整体缩放（参考"元宝"桌宠观感：紧凑小形象，约 90px 高）。 */
const PET_SCALE = 0.6;

/**
 * 身体核心命中矩形（**局部坐标**，未缩放）：身体 + 耳朵 + 头，
 * 不含尾巴与四周空白角——尾巴仍可见但不可点（穿透判定据此，见 updateHitRegion）。
 */
const CORE_LEFT = -68;
const CORE_TOP = -96;
const CORE_W = 136;
const CORE_H = 154;

/**
 * remnant（耳 + 眼）包围盒（**局部坐标**，未缩放）：
 * 双耳 poly x[-60,60] / y[-96,-50]，眼睛（eyesG 内）y[-27,-1]。
 * 边缘吸附休眠时身体/脸/腮红/鼻/尾全部隐藏，只留这部分并整体逆时针旋转 90°。
 * 命中区用旋转后矩形（宽高互换），故只需宽高与中心 Y（pivot）。
 */
const FACE_TOP = -96;
const FACE_W = 120;
const FACE_H = 95;
/** remnant 包围盒中心 Y：faceG 的 pivot——保证旋转 -90° 后 remnant 仍居中于窗口。 */
const FACE_CENTER_Y = FACE_TOP + FACE_H / 2;

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
  /** 身体 Graphics（含脸/腮红/鼻；休眠时隐藏）。 */
  private bodyG: Graphics | null = null;
  /** remnant 容器（双耳 + 眼睛；休眠时唯一可见部分，绕 pivot 旋转 -90°）。 */
  private faceG: Container | null = null;
  /** 边缘吸附休眠态（命中区切换为 remnant 旋转矩形）。 */
  private edgeSleep = false;

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
    root.scale.set(PET_SCALE * this.scaleX * bx, PET_SCALE * this.scaleY * by);

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

  // 视线数据源：`probe:cursor:move` 事件（C2 单点轮询）→ useSystemProbe → lookAt。
  // 开发期的 window mousemove 直连已移除，避免双数据源。

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
    // 居中基于实际画布尺寸（resizeTo: container → app.screen = 容器逻辑尺寸），
    // 窗口 A 与预览舞台(任意正方形) 形象均正中对齐、完整展示；
    // CANVAS_SIZE 仅作初始默认值保留。
    this.centerX = this.app.screen.width / 2;
    this.centerY = this.app.screen.height / 2;
    const petRoot = new Container();
    petRoot.position.set(this.centerX, this.centerY);
    petRoot.scale.set(PET_SCALE);
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

    // 身体 + 面部 + 腮红 + 鼻子（静态，绘制一次；**双耳已拆出到 faceG**）
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
      .fill(COLOR_BODY);
    this.bodyG = bodyG;

    // 眼睛容器（lookAt / 眨眼 / 表情作用于此）
    const eyesG = new Container();
    this.eyesBaseX = 0;
    this.eyesBaseY = -14;
    eyesG.position.set(this.eyesBaseX, this.eyesBaseY);
    const eyeL = new Graphics().roundRect(-36, -13, 20, 26, 10).fill(COLOR_INK);
    const eyeR = new Graphics().roundRect(16, -13, 20, 26, 10).fill(COLOR_INK);
    eyesG.addChild(eyeL, eyeR);
    this.eyesG = eyesG;

    // 双耳（独立 Graphics：休眠 remnant 的组成部分）
    const earsG = new Graphics()
      .poly([-60, -50, -36, -96, -18, -56])
      .fill(COLOR_BODY)
      .stroke({ width: 2, color: COLOR_BODY_DEEP, alpha: 0.8 })
      .poly([60, -50, 36, -96, 18, -56])
      .fill(COLOR_BODY)
      .stroke({ width: 2, color: COLOR_BODY_DEEP, alpha: 0.8 });

    // remnant 容器：耳 + 眼。pivot 对齐包围盒中心 → 逆时针旋转 90° 后仍居中不偏心。
    const faceG = new Container();
    faceG.pivot.set(0, FACE_CENTER_Y);
    faceG.addChild(earsG, eyesG);
    this.faceG = faceG;

    petRoot.addChild(tailG, bodyG, faceG);

    // ---- 命中区：身体核心矩形（挂载时算一次，帧内不再 getBounds） ----
    this.updateHitRegion();

    // ---- 画布尺寸变化（窗口 A 展开/收起）→ 重新居中 + 命中区重算 ----
    window.addEventListener("resize", this.relayout);

    // ---- 单一 Ticker（视线数据源由外部事件驱动 lookAt，见类头注释） ----
    app.ticker.add(this.tick, this);
  }

  /** 命中区 = 身体核心矩形（随居中位置换算到窗口坐标；尾巴与空白角不入命中区）。
   *  休眠态切换为 remnant 旋转矩形：宽高互换、以窗口中心居中（否则点不到耳眼唤醒）。 */
  private updateHitRegion(): void {
    if (this.edgeSleep) {
      const w = FACE_H * PET_SCALE;
      const h = FACE_W * PET_SCALE;
      this.hitRegion.x = this.centerX - w / 2;
      this.hitRegion.y = this.centerY - h / 2;
      this.hitRegion.width = w;
      this.hitRegion.height = h;
      return;
    }
    this.hitRegion.x = this.centerX + CORE_LEFT * PET_SCALE;
    this.hitRegion.y = this.centerY + CORE_TOP * PET_SCALE;
    this.hitRegion.width = CORE_W * PET_SCALE;
    this.hitRegion.height = CORE_H * PET_SCALE;
  }

  /**
   * 重新布局（窗口 A 展开/收起后）：延后一帧等 PIXI `resizeTo` 生效，
   * 再按新画布尺寸重新居中并重算命中区——形象屏幕位置不跳（窗口中心保持不变）。
   */
  private readonly relayout = (): void => {
    requestAnimationFrame(() => {
      const app = this.app;
      const root = this.petRoot;
      if (!app || !root) return;
      this.centerX = app.screen.width / 2;
      this.centerY = app.screen.height / 2;
      root.position.set(this.centerX, this.centerY);
      this.updateHitRegion();
    });
  };

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
      case "tsundere":
        eyes.scale.set(1.1, 0.55); // 眯眼别扭（happy 弯眼 × 微眯，C6 傲娇基调）
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

  /** 边缘吸附休眠视觉（D4.4）：只露耳 + 眼并整体逆时针旋转 90°；唤醒恢复完整形象。
   *  身体/脸/腮红/鼻/尾隐藏，remnant（faceG）绕 pivot 旋转 -π/2（PIXI 负角=逆时针）；
   *  命中区同步切换为 remnant 旋转矩形（见 updateHitRegion）。
   *  帧率（maxFPS）不在此管理——由 PetWindow 以 `probeIdle || sleepSnap` 单一真源统一驱动（E2.3①）。 */
  setEdgeSleep(sleeping: boolean): void {
    const face = this.faceG;
    if (!face) return;
    this.edgeSleep = sleeping;
    if (this.tailG) this.tailG.visible = !sleeping;
    if (this.bodyG) this.bodyG.visible = !sleeping;
    face.rotation = sleeping ? -Math.PI / 2 : 0;
    if (sleeping) this.setExpression("sleepy");
    else this.setExpression("neutral");
    this.updateHitRegion();
  }

  dispose(): void {
    const app = this.app;
    this.app = null;
    this.petRoot = null;
    this.eyesG = null;
    this.tailG = null;
    this.bodyG = null;
    this.faceG = null;
    window.removeEventListener("resize", this.relayout);
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

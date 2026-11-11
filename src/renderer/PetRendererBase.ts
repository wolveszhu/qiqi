/**
 * 宠物渲染器**共享基类**（为第二套形象「汪汪」而抽取）：形态与配色由子类提供，
 * 其余全部在此共享。
 *
 * 性能红线（code-standards §6 / 规范 §3）：
 * - 所有图形**绘制一次**；每帧只改 transform（scale/rotation/position）；
 * - Ticker 回调体内**零 new、零对象/数组字面量**（复用类字段）；
 * - PIXI 实例由普通类字段持有，**绝不进 Vue 响应式**；
 * - 卸载走 `destroy(true, { children: true })` 并移除全部监听。
 *
 * 契约：`IPetRenderer`（baseline §D）。子类只需两个钩子：
 * - `spec()`：几何常量（命中矩形 / 圆盘圆心 / 缩放 / 眼睛基准位）；
 * - `draw()`：一次性绘制的四层图层（尾 / 身体 / 双耳 / 眼睛容器）。
 *
 * 猫狗因此共用同一套 Ticker、物理、命中区与边缘吸附数学——任何一处物理改动只需改这里。
 */

import { Application, Container } from "pixi.js";

// Graphics 仅用于 `PetLayers` 的类型标注——绘制全部发生在子类，基类不构造 Graphics
import type { Graphics, Ticker } from "pixi.js";
import { breathScales, dampedRecovery, eyeOffset, jellyImpulse } from "./physics";
import type { HitRegion, IPetRenderer, PetAction, PetExpression } from "./IPetRenderer";
import { SNAP_SHOW_RATIO_DEFAULT, type SnapEdge } from "../utils/edgeSnap";

/** 画布逻辑尺寸初始默认值（mount 时按容器实际尺寸重算）。 */
const CANVAS_SIZE = 240;

/**
 * 贴边休眠命中区裁切结果的**有效性兜底**（纯函数，单测覆盖）。
 *
 * 裁切退化为 0 或负值（贴边比例 / 旋转参数边界）时回退整窗：绝不上报无效命中区，
 * 否则 Rust 穿透状态机会把桌宠锁在「整窗穿透」里，用户再也点不醒
 * （`click_through.rs::is_over_pet` 另有同向兜底，此处是上报端双保险）。
 */
export function ensureValidHitRegion(
  left: number,
  top: number,
  width: number,
  height: number,
  windowWidth: number,
  windowHeight: number,
): HitRegion {
  if (width > 0 && height > 0) {
    return { x: left, y: top, width, height };
  }
  return { x: 0, y: 0, width: windowWidth, height: windowHeight };
}

/** 动画常量（两套形象共用；集中定义，防散落魔法数）。 */
const BREATH_OMEGA = 0.0025; // rad/ms → 周期约 2.5s
const TAIL_OMEGA = 0.0021; // 尾巴摆动角频率
const TAIL_AMPLITUDE = 0.18; // 弧度
const BLINK_DURATION_MS = 140;
const BLINK_MIN_GAP_MS = 2600;
const BLINK_GAP_RANGE_MS = 2600;
const EXPRESSION_HOLD_MS = 700; // click 触发 happy 的持续时间

/**
 * 每套形象向基类提供的**几何描述**（不含绘制，绘制在 `draw()`）。
 *
 * `core` 与 `bodyCenterY` 是两份**必须各自标定**的量：`core` 决定点击穿透范围，
 * `bodyCenterY` 是"吸附只露半个圆"的圆心基准。照抄另一套形象的数值会导致
 * 躺倒时露出错误部位或点击判定偏移。
 */
export interface PetShapeSpec {
  /** 身体核心命中矩形（**局部坐标**，含身体+耳+头，不含尾巴与四周空白角）。 */
  core: { left: number; top: number; width: number; height: number };
  /** 形象"圆盘"中心 Y（**局部坐标**）：边缘吸附旋转与半圆裁切的圆心基准。 */
  bodyCenterY: number;
  /** 形象整体缩放。 */
  scale: number;
  /** 眼睛容器基准位置（**局部坐标**）：lookAt / 眨眼 / 表情以此为原点。 */
  eyes: { x: number; y: number };
}

/** 一次性绘制出的四个图层；顺序即 `headG.addChild(body, ears, eyes)` 的顺序。 */
export interface PetLayers {
  /** 尾巴容器：位置与 pivot 由子类设在尾根，每帧仅 rotation。 */
  tail: Container;
  /** 身体（毛发填充 + 面部 + 腮红 + 鼻 等，合并绘制一次）。 */
  body: Graphics;
  /** 双耳（猫尖立耳 / 狗垂耳）。 */
  ears: Graphics;
  /** 眼睛容器（内含两枚眼 Graphics），lookAt/眨眼/表情作用于此。 */
  eyes: Container;
}

export abstract class PetRendererBase implements IPetRenderer {
  // ---- PIXI 实例（普通字段，非响应式） ----
  private app: Application | null = null;
  private petRoot: Container | null = null;
  private eyesG: Container | null = null;
  private tailG: Container | null = null;
  /** 形象容器（身体毛发 + 脸 + 双耳 + 眼睛）：吸附休眠时整体旋转 ±90°。 */
  private headG: Container | null = null;
  /** 边缘吸附休眠态（形象外移半窗宽 + 命中区取可见半边）。 */
  private edgeSleep = false;

  // ---- 子类几何缓存（mount 时取一次；避免每帧调用抽象方法） ----
  private coreLeft = 0;
  private coreTop = 0;
  private coreWidth = 0;
  private coreHeight = 0;
  /** 形象核心包围盒中心 Y（**局部坐标**）：吸附旋转的旋转中心。 */
  private coreCenterY = 0;
  /** 形象"圆盘"中心 Y（**局部坐标**），见 `PetShapeSpec.bodyCenterY`。 */
  private bodyCenterY = 0;
  private petScale = 1;

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

  // 视线数据源：`probe:cursor:move` 事件（C2 单点轮询）→ useSystemProbe → lookAt。
  // 开发期的 window mousemove 直连已移除，避免双数据源。

  /** 子类几何常量（每套形象一份，互不干扰——尤其 `core` 与 `bodyCenterY` 必须各自标定）。 */
  protected abstract spec(): PetShapeSpec;

  /** 子类一次性绘制的图层（挂载时调用一次，帧内不再 new Graphics）。 */
  protected abstract draw(): PetLayers;

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
    root.scale.set(this.petScale * this.scaleX * bx, this.petScale * this.scaleY * by);

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

    // ---- 几何常量取一次（cats/dogs 各自标定） ----
    const spec = this.spec();
    this.coreLeft = spec.core.left;
    this.coreTop = spec.core.top;
    this.coreWidth = spec.core.width;
    this.coreHeight = spec.core.height;
    this.coreCenterY = spec.core.top + spec.core.height / 2;
    this.bodyCenterY = spec.bodyCenterY;
    this.petScale = spec.scale;
    this.eyesBaseX = spec.eyes.x;
    this.eyesBaseY = spec.eyes.y;

    // ---- 分层容器（仅一次） ----
    // 居中基于实际画布尺寸（resizeTo: container → app.screen = 容器逻辑尺寸），
    // 窗口 A 与预览舞台(任意正方形) 形象均正中对齐、完整展示；
    // CANVAS_SIZE 仅作初始默认值保留。
    this.centerX = this.app.screen.width / 2;
    this.centerY = this.app.screen.height / 2;
    const petRoot = new Container();
    petRoot.position.set(this.centerX, this.centerY);
    petRoot.scale.set(this.petScale);
    this.petRoot = petRoot;
    app.stage.addChild(petRoot);

    // ---- 子类绘制的图层 ----
    const layers = this.draw();
    this.tailG = layers.tail;

    const eyesG = layers.eyes;
    eyesG.position.set(this.eyesBaseX, this.eyesBaseY);
    this.eyesG = eyesG;

    // 形象容器：身体（毛发+脸+腮红+鼻）+ 双耳 + 眼睛。
    // **不可用 pivot**：PIXI 的 pivot 会永久平移子渲染（rendered = position + (child − pivot)），
    // awake 态会整体错位。旋转居中改由 setEdgeSleep 的 position 补偿实现。
    const headG = new Container();
    headG.addChild(layers.body, layers.ears, eyesG);
    this.headG = headG;

    petRoot.addChild(layers.tail, headG);

    // ---- 命中区：身体核心矩形（挂载时算一次，帧内不再 getBounds） ----
    this.updateHitRegion();

    // ---- 画布尺寸变化（窗口 A 展开/收起）→ 重新居中 + 命中区重算 ----
    window.addEventListener("resize", this.relayout);

    // ---- 单一 Ticker（视线数据源由外部事件驱动 lookAt，见类头注释） ----
    app.ticker.add(this.tick, this);
  }

  /**
   * 命中区 = 身体核心矩形（随居中位置换算到窗口坐标；尾巴与空白角不入命中区）。
   * 吸附休眠态：形象旋转 90° → 矩形**宽高互换**，中心仍为形象中心
   * （(centerX, centerY + coreCenterY·scale)），否则点不中唤醒。
   */
  private updateHitRegion(): void {
    if (this.edgeSleep) {
      const head = this.headG;
      if (!head) return;
      // 旋转后内容中心（窗口局部坐标）：rendered = P + R(θ)·C，C = (0, coreCenterY)
      const cos = Math.cos(head.rotation);
      const sin = Math.sin(head.rotation);
      const cx =
        this.centerX + (head.position.x - this.coreCenterY * sin) * this.petScale;
      const cy =
        this.centerY + (head.position.y + this.coreCenterY * cos) * this.petScale;
      // 旋转 90° → 宽高互换：半宽 = coreHeight/2，半高 = coreWidth/2
      const halfW = (this.coreHeight / 2) * this.petScale;
      const halfH = (this.coreWidth / 2) * this.petScale;
      // 只报朝屏幕内、落在窗口内的那半边（其余溢出窗口不可见）
      const left = Math.max(0, cx - halfW);
      const right = Math.min(this.centerX * 2, cx + halfW);
      const top = Math.max(0, cy - halfH);
      const bottom = Math.min(this.centerY * 2, cy + halfH);
      const width = Math.max(0, right - left);
      const height = Math.max(0, bottom - top);
      const region = ensureValidHitRegion(
        left,
        top,
        width,
        height,
        this.centerX * 2,
        this.centerY * 2,
      );
      // 逐字段赋值而非替换对象：`getHitRegion()` 复用同一对象，外部可能已持有引用。
      this.hitRegion.x = region.x;
      this.hitRegion.y = region.y;
      this.hitRegion.width = region.width;
      this.hitRegion.height = region.height;
      return;
    }
    this.hitRegion.x = this.centerX + this.coreLeft * this.petScale;
    this.hitRegion.y = this.centerY + this.coreTop * this.petScale;
    this.hitRegion.width = this.coreWidth * this.petScale;
    this.hitRegion.height = this.coreHeight * this.petScale;
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

  /**
   * 边缘吸附休眠视觉（D4.4）：把形象想象成一个圆——吸附态**只露出半个圆**。
   *
   * 实现（不出屏方案）：
   * - 窗口仍完全贴在屏幕内侧（`snap_target_position` 负责：left→x=mx，right→x=mx+mw−w）；
   * - 渲染层把整个形象（身体毛发 + 脸 + 耳 + 眼）沿**朝屏幕外**方向平移半个窗口宽，
   *   使形象圆心正好落在窗口贴边那侧的边线上 → 朝向屏幕内的半边留在窗口内可见（半圆 D 形），
   *   另一半溢出窗口被自然裁掉。
   * - 旋转方向：贴左 → **顺时针** +π/2；贴右 → **逆时针** −π/2，
   *   两者都让头部（耳 + 眼 + 脸）朝向屏幕内侧。
   *
   * 露出比例 `showRatio`：圆盘直径中被留在窗口内（可见）的比例；0.5 = 半个圆（默认）。
   * 圆心因此被推到距贴边一侧边线 `(showRatio − 0.5)·直径` 处。
   *
   * 数学：容器 pivot 为原点、rotation = θ 时，局部点 p 渲染到 `P + R(θ)·p`；
   * 令圆心 D=(0, bodyCenterY) 落在目标 T 上 → `P = T − R(θ)·D`，
   * 其中 R(θ)·D = (−d·sinθ, d·cosθ)，按 sin/cos 展开避免手算正负号出错。
   */
  setEdgeSleep(
    sleeping: boolean,
    side: SnapEdge | null = null,
    showRatio: number = SNAP_SHOW_RATIO_DEFAULT,
  ): void {
    const head = this.headG;
    if (!head) return;
    this.edgeSleep = sleeping;
    if (this.tailG) this.tailG.visible = !sleeping; // 躺姿下尾巴收起

    // 顺时针（贴左，头朝屏幕内）为 +π/2；逆时针（贴右）为 −π/2。
    const rotation = sleeping ? (side === "right" ? -Math.PI / 2 : Math.PI / 2) : 0;
    head.rotation = rotation;

    // 圆盘直径（旋转后沿水平方向的尺寸）：决定圆心偏离窗口边线的距离
    const discDiameter = this.coreHeight * this.petScale;
    // 圆心目标（窗口局部）：露出比例 r → 圆心距贴边一侧边线 (r − 0.5)·直径
    const inset = (showRatio - 0.5) * discDiameter;
    const targetWinX = sleeping
      ? side === "right"
        ? this.centerX * 2 - inset
        : inset
      : this.centerX;
    const targetWinY = this.centerY + this.bodyCenterY * this.petScale; // 纵向保持拖放高度

    // 换算到 petRoot 局部单位（父级缩放 scale）
    const targetX = (targetWinX - this.centerX) / this.petScale;
    const targetY = (targetWinY - this.centerY) / this.petScale;
    const cos = Math.cos(rotation);
    const sin = Math.sin(rotation);
    // P = T − R(θ)·D，其中 R(θ)·D = (−d·sinθ, d·cosθ)，d = bodyCenterY
    head.position.set(targetX + this.bodyCenterY * sin, targetY - this.bodyCenterY * cos);

    this.setExpression("neutral");
    this.updateHitRegion();
  }

  dispose(): void {
    const app = this.app;
    this.app = null;
    this.petRoot = null;
    this.eyesG = null;
    this.tailG = null;
    this.headG = null;
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

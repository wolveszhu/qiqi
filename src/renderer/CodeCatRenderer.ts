/**
 * 代码猫渲染器（C1 完整实现）：继承 `PetRendererBase`，只提供**几何常量 + 一次性绘制**。
 *
 * 通用逻辑（Application 生命周期 / 单一 Ticker / 物理 / 命中区 / 边缘吸附 / 销毁）
 * 已在基类 `PetRendererBase` 中，与新增的「汪汪」共用——任何物理改动只改基类。
 *
 * 纪律（code-standards §6 / 规范 §3）：
 * - 图形**绘制一次**（`draw()` 只在挂载时被调用一次），每帧只改 transform；
 * - 帧内零 `new`、零对象/数组字面量（Ticker 在基类，本类不参与每帧逻辑）；
 * - 本类不得持有 PIXI 实例——全部由基类以普通类字段持有。
 *
 * 契约：`IPetRenderer`（baseline §D）。
 */

import { Container, Graphics } from "pixi.js";

import type { PetLayers, PetShapeSpec } from "./PetRendererBase";
import { PetRendererBase } from "./PetRendererBase";

/** 形象整体缩放（参考"元宝"桌宠观感：紧凑小形象，约 90px 高）。 */
const PET_SCALE = 0.6;

/**
 * 身体核心命中矩形（**局部坐标**，未缩放）：身体 + 耳朵 + 头，
 * 不含尾巴与四周空白角——尾巴仍可见但不可点（穿透判定据此，见基类 updateHitRegion）。
 */
const CORE_LEFT = -68;
const CORE_TOP = -96;
const CORE_W = 136;
const CORE_H = 154;
/**
 * 形象"圆盘"中心 Y（**局部坐标**）= 身体圆角矩形中心（-56 + 114/2 = 1）。
 *
 * 半个圆的裁切基准：把圆盘圆心压到窗口贴边一侧的边线上，
 * 于是圆盘恰好一半在窗口内可见（另一半溢出被裁掉）；
 * 因头部在圆盘上半部，露出的正是耳 + 眼 + 脸 + 毛发。
 */
const BODY_CENTER_Y = -56 + 114 / 2;

/** 眼睛容器基准位置（**局部坐标**）。 */
const EYES_BASE_X = 0;
const EYES_BASE_Y = -14;

/**
 * 配色（**奶油黄**，照 `docs/design/pet-design/assets/pet-cat.jpg` 校正）。
 *
 * 此前误用绿色（正是设计稿标给「汪汪」的抹茶绿），与设计稿「猫猫 · 奶油黄」不符；
 * 那两个绿色常量已整体迁移给 `CodeDogRenderer`，调色板总量不变。
 * 纪律：只改色值，**任何坐标/形状/图层/动画参数都不许动**。
 */
const COLOR_BODY = 0xf2d9a2;
const COLOR_BODY_DEEP = 0xd9bc7e;
/** 额前刘海：参考图上是比面盘更浅的米白，沿用身体色会与耳/头糊成一片。 */
const COLOR_BANGS = 0xf8f2e2;
const COLOR_FACE = 0xfffef9;
const COLOR_INK = 0x403a2e;
const COLOR_BLUSH = 0xf5aeb9;
const COLOR_NOSE = 0xff9e5e;

export class CodeCatRenderer extends PetRendererBase {
  protected spec(): PetShapeSpec {
    return {
      core: { left: CORE_LEFT, top: CORE_TOP, width: CORE_W, height: CORE_H },
      bodyCenterY: BODY_CENTER_Y,
      scale: PET_SCALE,
      eyes: { x: EYES_BASE_X, y: EYES_BASE_Y },
    };
  }

  protected draw(): PetLayers {
    // 尾巴（独立容器：pivot 在根部，每帧仅 rotation）
    const tailG = new Container();
    tailG.position.set(58, 30);
    tailG.pivot.set(58, 30);
    const tail = new Graphics()
      .moveTo(58, 30)
      .bezierCurveTo(104, 6, 126, 58, 104, 96)
      .stroke({ width: 14, color: COLOR_BODY_DEEP, cap: "round" });
    tailG.addChild(tail);

    // 身体（毛发填充）+ 面部 + 腮红 + 鼻子 + 刘海（静态，绘制一次；刘海为独立浅色）
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
      .fill(COLOR_BANGS);

    // 双耳（独立 Graphics：与身体分开，便于整体旋转时保持绘制顺序）
    const earsG = new Graphics()
      .poly([-60, -50, -36, -96, -18, -56])
      .fill(COLOR_BODY)
      .stroke({ width: 2, color: COLOR_BODY_DEEP, alpha: 0.8 })
      .poly([60, -50, 36, -96, 18, -56])
      .fill(COLOR_BODY)
      .stroke({ width: 2, color: COLOR_BODY_DEEP, alpha: 0.8 });

    // 眼睛容器（lookAt / 眨眼 / 表情作用于此；基准位置由 spec.eyes 提供）
    const eyesG = new Container();
    const eyeL = new Graphics().roundRect(-36, -13, 20, 26, 10).fill(COLOR_INK);
    const eyeR = new Graphics().roundRect(16, -13, 20, 26, 10).fill(COLOR_INK);
    eyesG.addChild(eyeL, eyeR);

    return { tail: tailG, body: bodyG, ears: earsG, eyes: eyesG };
  }
}

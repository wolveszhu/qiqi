/**
 * 代码狗渲染器「汪汪」（第二套内置形象）：继承 `PetRendererBase`，只提供**几何常量 + 一次性绘制**。
 *
 * 与 `CodeCatRenderer` 同属 **Jelly-Toy 圆润玩具风**（圆角矩形身体、椭圆面盘、豆眼、腮红、小鼻），
 * **区别只在垂耳轮廓**（猫是尖立耳，狗是向下垂的长圆角耳）——这是两套形象的第一识别点。
 *
 * 通用逻辑（Application / 单一 Ticker / 物理 / 命中区 / 边缘吸附躺倒 / 销毁）全在基类，
 * 与猫共用同一份实现。
 *
 * 纪律（code-standards §6 / 规范 §3）：
 * - 图形**绘制一次**（`draw()` 只在挂载时调用一次），每帧只改 transform；
 * - 帧内零 `new`、零对象/数组字面量（Ticker 在基类，本类不参与每帧逻辑）；
 * - 本类不得持有 PIXI 实例——全部由基类以普通类字段持有。
 *
 * 契约：`IPetRenderer`（baseline §D）。
 */

import { Container, Graphics } from "pixi.js";

import type { PetLayers, PetShapeSpec } from "./PetRendererBase";
import { PetRendererBase } from "./PetRendererBase";

/** 形象整体缩放（与猫一致，保证两套形象观感尺寸相同）。 */
const PET_SCALE = 0.6;

/**
 * 身体核心命中矩形（**局部坐标**，未缩放）：身体 + 垂耳 + 头，不含尾巴与四周空白角。
 *
 * **按狗的体型单独标定**：
 * - 横向要覆盖**外凸的垂耳**（耳外沿 ±86 > 身体 ±68），否则点击耳朵点不中；
 * - 纵向顶端就是身体上沿（-56 加 3px 描边 → -58），垂耳不上冲，故比猫（含立耳 -96）矮。
 *
 * 这两个量决定点击穿透范围与"吸附只露半个圆"的圆心，照抄猫会导致躺倒时露出错误部位。
 */
const CORE_LEFT = -86;
const CORE_TOP = -58;
const CORE_W = 172;
const CORE_H = 116;

/** 垂耳（**第一识别点**）：从头顶两侧下垂、露到身体轮廓外的胶囊，左右对称。 */
const EAR_W = 30;
const EAR_TOP = -46;
const EAR_H = 92;
/**
 * 形象"圆盘"中心 Y（**局部坐标**）= 身体圆角矩形中心（-56 + 114/2 = 1）。
 * 与猫同值：狗的身体矩形沿用猫的尺寸，故半圆裁切基准一致。
 */
const BODY_CENTER_Y = -56 + 114 / 2;

/** 眼睛容器基准位置（**局部坐标**）：狗的豆眼略高于猫。 */
const EYES_BASE_X = 0;
const EYES_BASE_Y = -12;

/** 配色（**抹茶绿**，照 `docs/design/pet-design/assets/pet-dog.jpg`；即猫的旧色）。 */
const COLOR_BODY = 0x7ca65b;
const COLOR_BODY_DEEP = 0x5d8541;
const COLOR_FACE = 0xfffef9;
const COLOR_INK = 0x403a2e;
const COLOR_BLUSH = 0xf5aeb9;

export class CodeDogRenderer extends PetRendererBase {
  protected spec(): PetShapeSpec {
    return {
      core: { left: CORE_LEFT, top: CORE_TOP, width: CORE_W, height: CORE_H },
      bodyCenterY: BODY_CENTER_Y,
      scale: PET_SCALE,
      eyes: { x: EYES_BASE_X, y: EYES_BASE_Y },
    };
  }

  protected draw(): PetLayers {
    // 尾巴（独立容器：pivot 在根部，每帧仅 rotation；狗尾短粗一些）
    const tailG = new Container();
    tailG.position.set(58, 30);
    tailG.pivot.set(58, 30);
    const tail = new Graphics()
      .moveTo(58, 30)
      .bezierCurveTo(98, 14, 112, 52, 100, 82)
      .stroke({ width: 17, color: COLOR_BODY_DEEP, cap: "round" });
    tailG.addChild(tail);

    // 身体（毛发填充）+ 面盘 + 吻部 + 腮红 + 鼻头 + 嘴 + 额顶（静态，绘制一次）
    // 与猫同族的矮胖圆角矩形；**吻部**在面盘下方再叠一层白椭圆，让口鼻"凸"出来
    const bodyG = new Graphics()
      .roundRect(-68, -56, 136, 114, 46)
      .fill(COLOR_BODY)
      .stroke({ width: 3, color: COLOR_BODY_DEEP, alpha: 0.85 })
      .ellipse(0, -6, 54, 46)
      .fill(COLOR_FACE)
      .ellipse(0, 20, 26, 16)
      .fill(COLOR_FACE)
      .ellipse(-34, 12, 11, 7)
      .fill({ color: COLOR_BLUSH, alpha: 0.55 })
      .ellipse(34, 12, 11, 7)
      .fill({ color: COLOR_BLUSH, alpha: 0.55 })
      // 鼻头：狗是**大黑鼻**（区别于猫的小橘鼻）
      .ellipse(0, 12, 9, 7)
      .fill(COLOR_INK)
      // 嘴：一道下弯弧线——狗嘴比猫更有"笑感"
      .moveTo(-9, 24)
      .quadraticCurveTo(0, 31, 9, 24)
      .stroke({ width: 2.5, color: COLOR_INK, cap: "round" })
      .ellipse(0, -42, 30, 14)
      .fill(COLOR_BODY);

    // 垂耳（**与猫的尖立耳是本形象的第一识别点**）：
    // 从头顶两侧起向下垂至嘴角以下的胶囊，**必须露到身体轮廓之外**（身体 x 仅 ±68）——
    // 与身体同色时，只有靠改变整体剪影才能被看成耳朵，否则会糊成一个方块。
    const earsG = new Graphics()
      .roundRect(-86, EAR_TOP, EAR_W, EAR_H, EAR_W / 2)
      .fill(COLOR_BODY)
      .stroke({ width: 2, color: COLOR_BODY_DEEP, alpha: 0.8 })
      .roundRect(56, EAR_TOP, EAR_W, EAR_H, EAR_W / 2)
      .fill(COLOR_BODY)
      .stroke({ width: 2, color: COLOR_BODY_DEEP, alpha: 0.8 });

    // 眼睛容器（lookAt / 眨眼 / 表情作用于此；基准位置由 spec.eyes 提供）
    // 狗眼用**圆润椭圆**（猫是竖长胶囊），两种眼形也是一大区分点
    const eyesG = new Container();
    const eyeL = new Graphics().ellipse(-24, -14, 11, 13).fill(COLOR_INK);
    const eyeR = new Graphics().ellipse(24, -14, 11, 13).fill(COLOR_INK);
    eyesG.addChild(eyeL, eyeR);

    return { tail: tailG, body: bodyG, ears: earsG, eyes: eyesG };
  }
}

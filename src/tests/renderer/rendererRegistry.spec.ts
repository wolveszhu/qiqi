/**
 * 渲染器注册表单测（第二套内置形象「汪汪」）。
 *
 * 覆盖：两套内置形象均可实例化、未知 id 回落 cat、清单含两项、形状常量自洽。
 * 只测纯逻辑（注册表 + spec），不初始化 PIXI（无 WebGL 环境）。
 */

import { describe, expect, it } from "vitest";

import { CodeCatRenderer } from "../../renderer/CodeCatRenderer";
import { CodeDogRenderer } from "../../renderer/CodeDogRenderer";
import { createRenderer, listRenderers } from "../../renderer/factory";

describe("createRenderer", () => {
  it("code_cat → CodeCatRenderer", () => {
    expect(createRenderer("code_cat")).toBeInstanceOf(CodeCatRenderer);
  });

  it("code_dog → CodeDogRenderer（第二套内置形象已注册）", () => {
    expect(createRenderer("code_dog")).toBeInstanceOf(CodeDogRenderer);
  });

  it("未知 id 回落 code_cat（容错，不抛）", () => {
    expect(createRenderer("nope")).toBeInstanceOf(CodeCatRenderer);
    expect(createRenderer("")).toBeInstanceOf(CodeCatRenderer);
  });
});

describe("listRenderers", () => {
  it("恰含两套内置形象", () => {
    const ids = listRenderers();
    expect(ids).toContain("code_cat");
    expect(ids).toContain("code_dog");
    expect(ids).toHaveLength(2);
  });
});

/** 取子类的 spec（protected，测试内以索引访问读取）。 */
function specOf(r: object): {
  core: { left: number; top: number; width: number; height: number };
  bodyCenterY: number;
  scale: number;
  eyes: { x: number; y: number };
} {
  const spec = (r as { spec(): unknown }).spec.bind(r);
  return spec() as ReturnType<typeof specOf>;
}

describe("形状常量自洽", () => {
  it("两套形象的 core 矩形与圆盘圆心均合法", () => {
    for (const r of [new CodeCatRenderer(), new CodeDogRenderer()]) {
      const s = specOf(r);
      expect(s.core.width).toBeGreaterThan(0);
      expect(s.core.height).toBeGreaterThan(0);
      expect(s.scale).toBeGreaterThan(0);
      // 圆盘圆心须落在 body 矩形纵向范围内（body: -56 .. 58）
      expect(s.bodyCenterY).toBeGreaterThanOrEqual(-56);
      expect(s.bodyCenterY).toBeLessThanOrEqual(58);
    }
  });

  it("狗的 core 更矮更宽——垂耳外凸、立耳上冲（与猫不同，避免照抄）", () => {
    const cat = specOf(new CodeCatRenderer());
    const dog = specOf(new CodeDogRenderer());
    // 纵向：猫的尖立耳上冲到 -96，狗的垂耳不上冲 → 更矮
    expect(dog.core.top).toBeGreaterThan(cat.core.top);
    expect(dog.core.height).toBeLessThan(cat.core.height);
    // 横向：狗的垂耳露到身体轮廓（±68）之外 → 更宽，且须覆盖耳外沿
    expect(dog.core.width).toBeGreaterThan(cat.core.width);
    expect(dog.core.left).toBeLessThanOrEqual(-86);
    expect(dog.core.left + dog.core.width).toBeGreaterThanOrEqual(86);
  });
});

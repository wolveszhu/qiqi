/**
 * 贴边休眠命中区有效性兜底单测（2026-11-11 回归）。
 *
 * 覆盖：首次启动拖动桌宠后整窗点击穿透、无法恢复。根因之一是前端把休眠态
 * 命中区裁成 0 宽/0 高上报，Rust 旧逻辑据此回退「整窗穿透」并因翻转短路锁死。
 */

import { describe, expect, it } from "vitest";

import { ensureValidHitRegion } from "../../renderer/PetRendererBase";

describe("ensureValidHitRegion", () => {
  it("有效裁切原样保留", () => {
    expect(ensureValidHitRegion(10, 20, 100, 80, 240, 240)).toEqual({
      x: 10,
      y: 20,
      width: 100,
      height: 80,
    });
  });

  it("零宽 / 零高回退整窗，绝不产生无效命中区", () => {
    const fullWindow = { x: 0, y: 0, width: 240, height: 240 };
    expect(ensureValidHitRegion(0, 0, 0, 80, 240, 240)).toEqual(fullWindow);
    expect(ensureValidHitRegion(0, 0, 100, 0, 240, 240)).toEqual(fullWindow);
  });

  it("负宽高（裁切越界）同样回退整窗", () => {
    expect(ensureValidHitRegion(300, 0, -60, 80, 240, 240)).toEqual({
      x: 0,
      y: 0,
      width: 240,
      height: 240,
    });
  });
});

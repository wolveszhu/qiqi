/**
 * 渲染器注册表（规范 §3 / 接口契约 §5）：active_renderer 字符串 → 实例。
 *
 * 两套内置形象：`code_cat`（喵喵 · 奶油黄）与 `code_dog`（汪汪 · 抹茶绿），
 * 均继承 `PetRendererBase`，只提供几何与绘制。
 *
 * 新增形象只需在此登记一行 + 实现 `PetRendererBase` 子类；设置面板的选项卡清单
 * 由 `listRenderers()` 派生（Phase 3 的 `CustomSpriteRenderer` 同理）。
 */

import type { IPetRenderer } from "./IPetRenderer";
import { CodeCatRenderer } from "./CodeCatRenderer";
import { CodeDogRenderer } from "./CodeDogRenderer";
import { CustomSpriteRenderer } from "./CustomSpriteRenderer";
import { getCustomConfig, uuidFromFullId } from "./customAvatarRegistry";

type RendererFactory = () => IPetRenderer;

const registry = new Map<string, RendererFactory>([
  ["code_cat", () => new CodeCatRenderer()],
  ["code_dog", () => new CodeDogRenderer()],
]);

/** 按 active_renderer 创建实例；未知值回落 code_cat（容错，不抛）。 */
export function createRenderer(activeRenderer: string): IPetRenderer {
  const factory = registry.get(activeRenderer);
  if (factory) return factory();
  // 自定义形象：custom:<uuid> → 取注册表配置构造精灵渲染器
  const uuid = uuidFromFullId(activeRenderer);
  if (uuid) {
    const cfg = getCustomConfig(uuid);
    if (cfg) return new CustomSpriteRenderer(cfg);
  }
  const fallback = registry.get("code_cat");
  if (!fallback) {
    throw new Error(`渲染器注册表缺少 code_cat 基线: ${activeRenderer}`);
  }
  return fallback();
}

/** 已注册的渲染器 id 列表（D2 AvatarPanel 展示/禁用态用）。 */
export function listRenderers(): string[] {
  return [...registry.keys()];
}

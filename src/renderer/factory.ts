/**
 * 渲染器注册表（规范 §3 / 接口契约 §5）：active_renderer 字符串 → 实例。
 *
 * Phase 3 扩展（G4 指南）：CustomSpriteRenderer 实现 IPetRenderer 后在此注册一行即可。
 */

import type { IPetRenderer } from "./IPetRenderer";
import { CodeCatRenderer } from "./CodeCatRenderer";

type RendererFactory = () => IPetRenderer;

const registry = new Map<string, RendererFactory>([
  ["code_cat", () => new CodeCatRenderer()],
  // TODO(Phase3): ["custom_sprite", () => new CustomSpriteRenderer()]
]);

/** 按 active_renderer 创建实例；未知值回落 code_cat（容错，不抛）。 */
export function createRenderer(activeRenderer: string): IPetRenderer {
  const factory = registry.get(activeRenderer) ?? registry.get("code_cat");
  if (!factory) {
    throw new Error(`渲染器注册表缺少 code_cat 基线: ${activeRenderer}`);
  }
  return factory();
}

/** 已注册的渲染器 id 列表（D2 AvatarPanel 展示/禁用态用）。 */
export function listRenderers(): string[] {
  return [...registry.keys()];
}

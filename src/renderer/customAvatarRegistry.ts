/**
 * 自定义形象注册表（窗口 A 与窗口 B 共享的**内存**缓存）。
 *
 * 责任：在渲染器工厂 `createRenderer('custom:<uuid>')` 被调用前，确保对应配置已就绪。
 * - 启动 / 导入 / 删除后调用 `loadCustomAvatars()` 从 Rust 拉取列表 + 图片字节并填充；
 * - 微调滑杆拖动时仅 `updateCustomConfigInMemory` 改内存（即时预览），落库走防抖命令。
 *
 * 纪律（规范 §3 / §I）：
 * - 不进 Vue 响应式：注册表是模块级 `Map` 普通变量，渲染器由普通变量持有；
 * - 图片 DataURL 仅驻内存（不落 `localStorage` / 不进 reactive），契合密钥与隐私红线。
 */

import { listCustomAvatars, readCustomAvatar } from "../api/customAvatar";

/** 自定义形象配置（Rust 落库字段 + 前端运行期 dataUrl，一一对应）。 */
export interface CustomAvatarConfig {
  /** UUID 部分（不含 `custom:` 前缀）。 */
  id: string;
  /** 完整 id，即 `factory.ts` / `active_renderer` 用的 `custom:<uuid>`。 */
  fullId: string;
  name: string;
  /** `data:image/png;base64,...`（仅内存，用于 PIXI `Assets.load`）。 */
  dataUrl: string;
  /** 整体缩放倍率（1 = 按舞台自适应铺满基准，见 CustomSpriteRenderer.STAGE_FIT_RATIO）。 */
  scale: number;
  /** 圆盘中心 X（归一化 0..1，默认 0.5）。 */
  anchorX: number;
  /** 圆盘中心 Y（归一化 0..1，默认 0.5）。 */
  anchorY: number;
}

const CUSTOM_PREFIX = "custom:";

const registry = new Map<string, CustomAvatarConfig>();

/** 取单个配置（按 UUID）。工厂创建渲染器时调用。 */
export function getCustomConfig(uuid: string): CustomAvatarConfig | undefined {
  return registry.get(uuid);
}

/** 列表全部完整 id（`custom:<uuid>`），供画廊与工厂判定。 */
export function listCustomAvatarIds(): string[] {
  return [...registry.values()].map((c) => c.fullId);
}

/** 列表全部配置，供画廊渲染。 */
export function listCustomConfigs(): CustomAvatarConfig[] {
  return [...registry.values()];
}

/** 即时更新内存中的几何/名称（预览用，不等 DB 落库）。 */
export function updateCustomConfigInMemory(
  uuid: string,
  patch: Partial<Pick<CustomAvatarConfig, "scale" | "anchorX" | "anchorY" | "name">>,
): void {
  const c = registry.get(uuid);
  if (c) Object.assign(c, patch);
}

/** 启动 / 导入 / 删除后刷新：拉列表并对每项取图片字节填充缓存（清空后整体替换）。 */
export async function loadCustomAvatars(): Promise<void> {
  const metas = await listCustomAvatars();
  const next = new Map<string, CustomAvatarConfig>();
  await Promise.all(
    metas.map(async (m) => {
      try {
        const dataUrl = await readCustomAvatar(m.id);
        next.set(m.id, {
          id: m.id,
          fullId: `${CUSTOM_PREFIX}${m.id}`,
          name: m.name,
          dataUrl,
          scale: m.scale,
          anchorX: m.anchorX,
          anchorY: m.anchorY,
        });
      } catch (e) {
        console.warn(`[avatar] 读取自定义形象图片失败 id=${m.id}`, e);
      }
    }),
  );
  registry.clear();
  for (const [k, v] of next) registry.set(k, v);
}

/** 导入成功后即时插入单条（避免整表重读图片字节）。 */
export function addCustomConfig(cfg: CustomAvatarConfig): void {
  registry.set(cfg.id, cfg);
}

/** 删除后从缓存移除。 */
export function removeCustomConfig(uuid: string): void {
  registry.delete(uuid);
}

/** 由完整 id 解析 UUID（非 custom: 前缀返回 null）。 */
export function uuidFromFullId(fullId: string): string | null {
  return fullId.startsWith(CUSTOM_PREFIX) ? fullId.slice(CUSTOM_PREFIX.length) : null;
}

/**
 * 自定义形象命令封装（Phase 3：自定义形象导入）。
 *
 * 边界红线（baseline §G）：前端只经 `tauri-plugin-dialog` 取**路径字符串**，
 * 真实文件的读取 / 拷贝 / 落库全部在 Rust 命令内（`std::fs` + SQLCipher）——本文件
 * 不接触任何文件系统 API。图片回传为 `data:image/png;base64,...`，前端仅用于创建
 * PIXI 纹理，绝不落本地存储（接口契约 §2 custom_*）。
 */

import { callCommand } from "./client";

/** 自定义形象元数据（与 Rust `CustomAvatarMetaDto` 对齐，camelCase）。 */
export interface CustomAvatarMeta {
  id: string;
  name: string;
  scale: number;
  anchorX: number;
  anchorY: number;
}

/** 导入自定义形象：name + 源文件路径（来自 dialog open）→ 返回元数据（不含图片字节）。 */
export async function importCustomAvatar(
  name: string,
  srcPath: string,
): Promise<CustomAvatarMeta> {
  return callCommand<CustomAvatarMeta>("import_custom_avatar", { name, srcPath });
}

/** 列出全部自定义形象（元数据，不含图片字节）。 */
export async function listCustomAvatars(): Promise<CustomAvatarMeta[]> {
  return callCommand<CustomAvatarMeta[]>("list_custom_avatars");
}

/** 读取自定义形象图片（返回 base64 DataURL，供 PIXI 建纹理）。 */
export async function readCustomAvatar(id: string): Promise<string> {
  return callCommand<string>("read_custom_avatar", { id });
}

/** 更新缩放与锚点（微调 UI 实时写回）。 */
export async function updateCustomAvatarConfig(
  id: string,
  scale: number,
  anchorX: number,
  anchorY: number,
): Promise<void> {
  await callCommand("update_custom_avatar_config", { id, scale, anchorX, anchorY });
}

/** 重命名自定义形象。 */
export async function renameCustomAvatar(id: string, name: string): Promise<void> {
  await callCommand("rename_custom_avatar", { id, name });
}

/** 删除自定义形象（若正被使用，Rust 回退 code_cat 并 emit 重建）。 */
export async function deleteCustomAvatar(id: string): Promise<void> {
  await callCommand("delete_custom_avatar", { id });
}

/**
 * 更新检测命令封装（规范 §3：唯一 invoke 出口 = `callCommand`）。
 *
 * 契约：Rust `UpdateInfo` 以 camelCase 序列化（`available/current/latest/notes/downloadUrl`）。
 */

import { callCommand } from "./client";

/** 更新信息（与 Rust `UpdateInfo` camelCase 契约对应）。 */
export interface UpdateInfo {
  /** 是否存在可更新版本。 */
  available: boolean;
  /** 当前安装版本。 */
  current: string;
  /** 远端最新版本。 */
  latest: string;
  /** 更新说明（多行）。 */
  notes?: string | null;
  /** 安装包下载地址。 */
  downloadUrl?: string | null;
}

/** 检测更新：拉取 latest.json 并比较版本；失败向上抛 AppError（前端友好提示）。 */
export async function checkUpdate(): Promise<UpdateInfo> {
  return callCommand<UpdateInfo>("check_update");
}

/**
 * settings 命令封装（规范 §3：唯一 invoke 出口 = `callCommand`）。
 *
 * 红线：本模块**不缓存、不落盘**任何明文 Key——Key 只经 IPC 单向流向 Rust。
 */

import { callCommand } from "./client";
import type { SaveTierPayload, SettingsSnapshot } from "./types";

/** 读取设置快照（回包含 hasKey 与末 4 位指纹，绝无明文 Key）。 */
export async function getTyped(): Promise<SettingsSnapshot> {
  return callCommand<SettingsSnapshot>("settings_get_typed");
}

/** 保存单档配置（main / memory / fallback）。 */
export async function saveTyped(
  tier: "main" | "memory" | "fallback",
  payload: SaveTierPayload,
): Promise<void> {
  await callCommand("settings_save_typed", { tier, ...payload });
}

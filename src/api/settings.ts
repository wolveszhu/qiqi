/**
 * settings 命令封装（规范 §3：唯一 invoke 出口 = `callCommand`）。
 *
 * 红线：本模块**不缓存、不落盘**任何明文 Key——Key 只经 IPC 单向流向 Rust。
 *
 * 参数约定（Tauri 2）：invoke 按**参数名**传值。Rust 侧签名为
 * `settings_save_typed(tier, payload)`，故此处必须传 `{ tier, payload }`，
 * 不能把 payload 字段展开到顶层（否则 Rust 侧缺 `payload` 字段导致参数校验失败）。
 */

import { callCommand } from "./client";
import type {
  CustomModelDto,
  DndConfig,
  ProactiveFrequency,
  ProviderPreset,
  SaveCustomModelPayload,
  SaveTierPayload,
  SettingsSnapshot,
  TierName,
} from "./types";

/** 读取设置快照（回包含 hasKey 与末 4 位指纹，绝无明文 Key）。 */
export async function getTyped(): Promise<SettingsSnapshot> {
  return callCommand<SettingsSnapshot>("settings_get_typed");
}

/** 保存单档配置（main / memory / fallback）。 */
export async function saveTyped(tier: TierName, payload: SaveTierPayload): Promise<void> {
  await callCommand("settings_save_typed", { tier, payload });
}

/** 保存勿扰与频率档位（type=2 行；dnd 与 frequency 同段持久化）。 */
export async function saveDnd(
  dnd: DndConfig,
  frequency: ProactiveFrequency,
): Promise<void> {
  await callCommand("settings_save_dnd", { dnd, frequency });
}

/** 切换额度模式（1=内置额度 / 2=BYOK；BYOK 需 main 档已配 Key，否则报 Config 错误）。 */
export async function setQuotaMode(mode: 1 | 2): Promise<void> {
  await callCommand("settings_set_quota_mode", { mode });
}

/** 服务商预设列表（C3：Rust 单一真源经 IPC 下发，禁止前端硬编码 URL）。 */
export async function listProviders(): Promise<ProviderPreset[]> {
  return callCommand<ProviderPreset[]>("settings_list_providers");
}

/** 自定义模型列表（按排序；Key 脱敏回传）。 */
export async function listCustomModels(): Promise<CustomModelDto[]> {
  return callCommand<CustomModelDto[]>("settings_list_custom_models");
}

/** 保存自定义模型（id 缺省=新建；返回保存后的 id）。 */
export async function saveCustomModel(payload: SaveCustomModelPayload): Promise<number> {
  return callCommand<number>("settings_save_custom_model", { payload });
}

/** 删除自定义模型。 */
export async function deleteCustomModel(id: number): Promise<void> {
  await callCommand("settings_delete_custom_model", { id });
}

/** 上移/下移一位（调整编排优先级）。 */
export async function moveCustomModel(id: number, up: boolean): Promise<void> {
  await callCommand("settings_move_custom_model", { id, up });
}

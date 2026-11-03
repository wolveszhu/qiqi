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
  SaveSearchSettings,
  SearchSettings,
  SettingsSnapshot,
} from "./types";

/** 读取设置快照（回包含 hasKey 与末 4 位指纹，绝无明文 Key）。 */
export async function getTyped(): Promise<SettingsSnapshot> {
  return callCommand<SettingsSnapshot>("settings_get_typed");
}

/** 保存勿扰与频率档位（type=2 行；dnd 与 frequency 同段持久化）。 */
export async function saveDnd(
  dnd: DndConfig,
  frequency: ProactiveFrequency,
): Promise<void> {
  await callCommand("settings_save_dnd", { dnd, frequency });
}

/** 切换额度模式（1=内置额度 / 2=BYOK；BYOK 需存在已启用且已填 Key 的自定义平台，否则报 Config 错误）。 */
export async function setQuotaMode(mode: 1 | 2): Promise<void> {
  await callCommand("settings_set_quota_mode", { mode });
}

/** 边缘吸附休眠开关（type=3 行；缺省 true）。 */
export async function setSnapSleep(enabled: boolean): Promise<void> {
  await callCommand("settings_set_snap_sleep", { enabled });
}

/** 吸附态露出比例（type=3 行；后端 clamp 到 [MIN, MAX]，返回落库值）。 */
export async function setSnapRatio(ratio: number): Promise<number> {
  return callCommand<number>("settings_set_snap_ratio", { ratio });
}

/** 服务商预设列表（C3：Rust 单一真源经 IPC 下发，禁止前端硬编码 URL）。 */
export async function listProviders(): Promise<ProviderPreset[]> {
  return callCommand<ProviderPreset[]>("settings_list_providers");
}

/** 读取联网搜索开关（对话偏好；缺省 false）。 */
export async function getWebSearch(): Promise<boolean> {
  return callCommand<boolean>("settings_get_web_search");
}

/** 写入联网搜索开关（合并且幂等，跨重启持久化）。 */
export async function setWebSearch(enabled: boolean): Promise<void> {
  await callCommand("settings_set_web_search", { enabled });
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

/** 读取联网搜索服务配置（Key 脱敏；未配置 Key 时 hasKey=false）。 */
export async function getSearchSettings(): Promise<SearchSettings> {
  return callCommand<SearchSettings>("search_settings_get");
}

/** 保存联网搜索服务配置（apiKey 缺省=不改动存量 Key，空串=清空）。 */
export async function saveSearchSettings(payload: SaveSearchSettings): Promise<void> {
  await callCommand("search_settings_save", { payload });
}

/** 上移/下移一位（调整编排优先级）。 */
export async function moveCustomModel(id: number, up: boolean): Promise<void> {
  await callCommand("settings_move_custom_model", { id, up });
}

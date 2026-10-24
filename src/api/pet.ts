/**
 * pet 命令封装：桌宠资料读写；好感度字段只读透出（结算属 C5）。
 *
 * 参数约定（Tauri 2）：invoke 按**参数名**传值。Rust 侧签名为
 * `pet_update_state(payload)`，故此处传 `{ payload }` 而非直接传 payload。
 */

import { callCommand } from "./client";
import type {
  AffectionChangedPayload,
  AffectionLogDto,
  AffectionSettleInput,
  PetStateDto,
  UpdatePetPayload,
} from "./types";

/** 读取桌宠状态。 */
export async function getState(): Promise<PetStateDto> {
  return callCommand<PetStateDto>("pet_get_state");
}

/** 更新桌宠资料（petName / activeRenderer / rendererConfig）。 */
export async function updateState(payload: UpdatePetPayload): Promise<PetStateDto> {
  return callCommand<PetStateDto>("pet_update_state", { payload });
}

/** 好感度结算信号（checkIn/petStroke 由前端触发；deepChat/resonance 由 C4 内部调用）。 */
export async function settleAffection(
  input: AffectionSettleInput,
): Promise<AffectionChangedPayload> {
  return callCommand<AffectionChangedPayload>("pet_settle_affection", { signal: input });
}

/** 好感度流水查询（D2 看板；limit 分页）。 */
export async function listAffectionLogs(limit?: number): Promise<AffectionLogDto[]> {
  return callCommand<AffectionLogDto[]>("pet_list_affection_logs", { limit });
}

/**
 * 休眠衰减锚点（D4.4）：enter=入睡前结清累计衰减；exit=赦免休眠天数（不掉分）。
 * action 非法时后端返回 Config 错误。
 */
export async function sleepDecay(action: "enter" | "exit"): Promise<void> {
  await callCommand("pet_sleep_decay", { action });
}

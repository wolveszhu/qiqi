/**
 * pet 命令封装：桌宠资料读写；好感度字段只读透出（结算属 C5）。
 */

import { callCommand } from "./client";
import type { PetStateDto, UpdatePetPayload } from "./types";

/** 读取桌宠状态。 */
export async function getState(): Promise<PetStateDto> {
  return callCommand<PetStateDto>("pet_get_state");
}

/** 更新桌宠资料（petName / activeRenderer / rendererConfig）。 */
export async function updateState(payload: UpdatePetPayload): Promise<PetStateDto> {
  return callCommand<PetStateDto>("pet_update_state", payload);
}

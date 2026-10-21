/**
 * 桌宠状态镜像（规范 §3：真源在 SQLCipher，前端只读缓存 + 写回命令）。
 * MVP 不引 Pinia（目录规范 §3 state/ 决策），接口与 Pinia store 兼容便于后续迁移。
 */

import { reactive } from "vue";

import type { LogItem, PetStateDto } from "../api/types";

export interface PetState extends PetStateDto {
  /** 当前渲染器 id（factory 注册表键；Phase 3 扩展）。 */
  activeRenderer: string;
  /** 好感度流水（D2 看板，limit 分页）。 */
  logs: LogItem[];
}

const state = reactive<PetState>({
  petName: "喵喵",
  relationshipLevel: 1,
  affectionPoints: 0,
  lastInteraction: null,
  activeRenderer: "code_cat",
  logs: [],
});

/** 以 pet_get_state 返回刷新（TODO(B5) 接线）。 */
export function applyPetState(dto: PetStateDto): void {
  state.petName = dto.petName;
  state.relationshipLevel = dto.relationshipLevel;
  state.affectionPoints = dto.affectionPoints;
  state.lastInteraction = dto.lastInteraction;
}

export function setPetLogs(logs: LogItem[]): void {
  state.logs = logs;
}

export const petStore = state;

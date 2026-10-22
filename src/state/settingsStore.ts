/**
 * 设置状态镜像（规范 §3：真源在 app_settings 表）。
 *
 * TODO(B5/C3): getTyped/saveTyped 接线；TODO(D4): settings:dnd:changed 双向同步。
 */

import { reactive } from "vue";

import type { DndConfig, ProactiveFrequency, SettingsSnapshot } from "../api/types";

export interface SettingsState {
  snapshot: SettingsSnapshot | null;
  /** 主动频率档位（C6 引擎读取）。 */
  frequency: ProactiveFrequency;
}

const state = reactive<SettingsState>({
  snapshot: null,
  frequency: "normal",
});

/** 以 settings_get_typed 返回刷新（含 dnd + frequency 同步到本地即时读取态）。 */
export function applySnapshot(snapshot: SettingsSnapshot): void {
  state.snapshot = snapshot;
  if (snapshot.frequency) {
    state.frequency = snapshot.frequency;
  }
}

/** dnd 本地即时更新（勿扰开关/时段编辑后调用，落库前乐观刷新）。 */
export function applyDndLocal(dnd: DndConfig): void {
  if (state.snapshot) {
    state.snapshot.dnd = dnd;
  }
}

export function setFrequency(frequency: ProactiveFrequency): void {
  state.frequency = frequency;
}

export const settingsStore = state;

/**
 * 设置状态镜像（规范 §3：真源在 app_settings 表）。
 *
 * D4：新增 `sleepSnap`（边缘吸附休眠态，内存态、正交勿扰，主动引擎 skip）；
 * `settings:dnd:changed` 双向同步由组件 emit/listen 驱动（双窗口独立 webview，前端 store 不共享）。
 *
 * 对话已迁至窗口 B「对话」Tab，故 `chatInputVisible`（窗口 A 运行时 UI 态）随之移除。
 */

import { reactive } from "vue";

import type { DndConfig, ProactiveFrequency, SettingsSnapshot } from "../api/types";

export interface SettingsState {
  snapshot: SettingsSnapshot | null;
  /** 主动频率档位（C6 引擎读取）。 */
  frequency: ProactiveFrequency;
  /** 边缘吸附休眠态（D4：内存态，正交勿扰，主动引擎 skip）。 */
  sleepSnap: boolean;
}

const state = reactive<SettingsState>({
  snapshot: null,
  frequency: "normal",
  sleepSnap: false,
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

/** 边缘吸附休眠态切换（D4.4：仅内存态，重启复位）。 */
export function setSleepSnap(value: boolean): void {
  state.sleepSnap = value;
}

export const settingsStore = state;

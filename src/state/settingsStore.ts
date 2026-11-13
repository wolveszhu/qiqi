/**
 * 设置状态镜像（规范 §3：真源在 app_settings 表）。
 *
 * D4.5：`sleepSnap`（已吸附态，系统默认行为）与 `snapSilent`（贴边静默态，受开关控制）
 * 是两个正交状态，主动引擎只读后者，边缘吸附不再受开关影响；
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
  /** 已贴边吸附态（D4.5：内存态，系统级默认行为，与静默开关无关）。 */
  sleepSnap: boolean;
  /** 贴边静默态（D4.5：内存态 = 已吸附 && 开关开启；主动引擎与交互门控读它）。 */
  snapSilent: boolean;
  /** 贴边静默开关（type=3 持久化；历史键名 `snapSleepEnabled`，语义已收窄为仅静默）。 */
  snapSleepEnabled: boolean;
  /** 吸附态露出比例（type=3 持久化；缺省 0.5，快照加载后覆盖）。 */
  snapShowRatio: number;
}

const state = reactive<SettingsState>({
  snapshot: null,
  frequency: "normal",
  sleepSnap: false,
  snapSilent: false,
  snapSleepEnabled: true,
  snapShowRatio: 0.5,
});

/** 以 settings_get_typed 返回刷新（含 dnd + frequency 同步到本地即时读取态）。 */
export function applySnapshot(snapshot: SettingsSnapshot): void {
  state.snapshot = snapshot;
  if (snapshot.frequency) {
    state.frequency = snapshot.frequency;
  }
  // 贴边静默开关（缺省 true 由后端保证；只影响免打扰，不影响吸附）
  state.snapSleepEnabled = snapshot.snapSleepEnabled !== false;
  // 吸附态露出比例（后端 clamp；非法值回落默认 0.5）
  state.snapShowRatio = Number.isFinite(snapshot.snapShowRatio)
    ? snapshot.snapShowRatio
    : 0.5;
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

/** 吸附态切换（D4.5：仅内存态，重启复位；静默态由 `snapSilent` 单独维护）。 */
export function setSleepSnap(value: boolean): void {
  state.sleepSnap = value;
}

export const settingsStore = state;

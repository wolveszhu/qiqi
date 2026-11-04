/**
 * 更新状态 composable（轻量自建远程更新）。
 *
 * 单例状态：设置窗口在挂载时调用 `initUpdateWatcher()` 注册一次全局监听器，
 * 后台周期检查（Rust `spawn_update_checker`）发现新版本时 emit `app:update-available`，
 * 此处写入 `updateInfo`；「关于」面板与全局横幅共享同一份状态，无需重复监听。
 *
 * 边界：本模块只持有响应式状态与「检查/关闭」动作，不渲染 UI；UI 由各组件消费。
 */

import { ref, computed, type Ref, type ComputedRef } from "vue";
import { listen } from "@tauri-apps/api/event";

import { checkUpdate, type UpdateInfo } from "../api/update";

/** 远端更新信息（null = 尚未检查/检查失败未覆盖）。 */
const updateInfo = ref<UpdateInfo | null>(null);
/** 手动检查进行中。 */
const checking = ref(false);
/** 最近一次手动检查的错误信息（网络/解析失败等）。 */
const lastError = ref<string | null>(null);
/** 用户是否已忽略本次横幅（不影响「关于」面板内的信息展示）。 */
const dismissed = ref(false);

/** 是否应展示「主动提醒」（有新版本且未被忽略）。 */
const hasUpdate = computed(
  () => updateInfo.value?.available === true && !dismissed.value,
);

let listenerRegistered = false;

/** 注册后台更新事件监听（幂等；失败静默，不阻断窗口）。 */
function registerListener(): void {
  if (listenerRegistered) return;
  listenerRegistered = true;
  void listen<UpdateInfo>("app:update-available", (e) => {
    updateInfo.value = e.payload;
    dismissed.value = false;
  }).catch(() => {});
}

/** 手动检查更新（「关于」面板的「检查更新」按钮）。 */
async function checkNow(): Promise<void> {
  checking.value = true;
  lastError.value = null;
  try {
    updateInfo.value = await checkUpdate();
    dismissed.value = false;
  } catch (e: unknown) {
    lastError.value = e instanceof Error ? e.message : String(e);
  } finally {
    checking.value = false;
  }
}

/** 忽略本次主动提醒（横幅/标记隐藏；「关于」面板信息仍在）。 */
function dismiss(): void {
  dismissed.value = true;
}

/** 在设置窗口挂载时调用一次，确保后台更新事件被监听。 */
export function initUpdateWatcher(): void {
  registerListener();
}

/** 消费更新状态与动作（组件层）。 */
export interface UseUpdate {
  updateInfo: Ref<UpdateInfo | null>;
  checking: Ref<boolean>;
  lastError: Ref<string | null>;
  hasUpdate: ComputedRef<boolean>;
  checkNow: () => Promise<void>;
  dismiss: () => void;
}

export function useUpdate(): UseUpdate {
  return { updateInfo, checking, lastError, hasUpdate, checkNow, dismiss };
}

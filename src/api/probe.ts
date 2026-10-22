/**
 * probe 封装（C2/C6）：探针事件订阅 + 快照 + 主动话术生成。
 *
 * 纪律（B5 规范 §3）：`listen` 只允许出现在本文件与 composables 的
 * `useSystemProbe` / `useProactive`，组件不得直接 listen。
 */

import { listen } from "@tauri-apps/api/event";

import { callCommand } from "./client";
import type {
  CursorPayload,
  ForegroundPayload,
  IdlePayload,
  ProbeSnapshot,
  ProactiveShowPayload,
} from "./types";
import type { UnlistenFn } from "@tauri-apps/api/event";

/** 订阅鼠标坐标事件（Rust 单点轮询 100ms 广播）。 */
export async function onCursorMove(
  cb: (x: number, y: number) => void,
): Promise<UnlistenFn> {
  return listen<CursorPayload>("probe:cursor:move", (event) => {
    cb(event.payload.x, event.payload.y);
  });
}

/** 订阅快捷键唤起的输入框切换事件（C2.7 Ctrl+Shift+P → D1 消费）。 */
export async function onChatToggle(cb: () => void): Promise<UnlistenFn> {
  return listen("window:chat:toggle", () => cb());
}

/** 订阅空闲翻转事件（C6：30min 无操作/回归，仅翻转到达）。 */
export async function onIdleChanged(cb: (p: IdlePayload) => void): Promise<UnlistenFn> {
  return listen<IdlePayload>("probe:idle:changed", (event) => {
    cb(event.payload);
  });
}

/** 订阅前台进程变化事件（C6：仅进程名，同名不重复发）。 */
export async function onForegroundChanged(
  cb: (p: ForegroundPayload) => void,
): Promise<UnlistenFn> {
  return listen<ForegroundPayload>("probe:foreground:changed", (event) => {
    cb(event.payload);
  });
}

/** 订阅主动话术事件（C6.4：Rust 短生成完毕 → 气泡展示）。 */
export async function onProactiveShow(
  cb: (p: ProactiveShowPayload) => void,
): Promise<UnlistenFn> {
  return listen<ProactiveShowPayload>("proactive:show", (event) => {
    cb(event.payload);
  });
}

/** 读取探针快照（一次读全量缓存，非订阅场景）。 */
export async function getProbeSnapshot(): Promise<ProbeSnapshot> {
  return callCommand<ProbeSnapshot>("probe_get_snapshot");
}

/** 请求 Rust 生成主动话术（非流式短生成；结果经 proactive:show 事件推送）。 */
export async function proactiveGenerate(tsundere: boolean): Promise<void> {
  await callCommand("proactive_generate", { tsundere });
}

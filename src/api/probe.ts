/**
 * probe 封装（C2）：cursor 事件订阅（B5 规范 §3：`listen` 只允许出现在
 * 本文件与 composables 的 `useSystemProbe`，组件不得直接 listen）。
 */

import { listen } from "@tauri-apps/api/event";

import type { UnlistenFn } from "@tauri-apps/api/event";
import type { CursorPayload } from "./types";

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

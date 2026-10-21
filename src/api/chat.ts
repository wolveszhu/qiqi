/**
 * chat 命令封装：B5 后端为 **echo 桩**（原文分片回吐），C4 替换实现后本模块零改动。
 *
 * 注意：文本流经 `chat:stream:delta` 事件推送，命令调用本身只负责发起。
 */

import { callCommand } from "./client";

/** 发起一次对话（流式结果经事件返回）。sessionId 由调用方生成（crypto.randomUUID()）。 */
export async function sendStream(sessionId: string, text: string): Promise<void> {
  await callCommand("chat_send_stream", { sessionId, text });
}

/** 中止指定会话。 */
export async function abort(sessionId: string): Promise<void> {
  await callCommand("chat_abort", { sessionId });
}

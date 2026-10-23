/**
 * chat 命令封装：B5 后端为 **echo 桩**（原文分片回吐），C4 替换实现后本模块零改动。
 *
 * 注意：文本流经 `chat:stream:delta` 事件推送，命令调用本身只负责发起。
 */

import { callCommand } from "./client";
import type { ChatMessageDto, ChatSessionDto, FilePayload } from "./types";

/** 发起一次对话（流式结果经事件返回）。sessionId 由调用方生成（crypto.randomUUID()）。
 *  attachments 可选（D1.5 多模态；缺省纯文本，向后兼容）。 */
export async function sendStream(
  sessionId: string,
  text: string,
  attachments: FilePayload[] = [],
): Promise<void> {
  await callCommand("chat_send_stream", { sessionId, text, attachments });
}

/** 解析拖入文件（Rust 读文件边界隔离；返回结构化负载，超限/非法 → AppError）。 */
export async function resolveDroppedFiles(paths: string[]): Promise<FilePayload[]> {
  return callCommand<FilePayload[]>("chat_resolve_dropped_files", { paths });
}

/** 中止指定会话。 */
export async function abort(sessionId: string): Promise<void> {
  await callCommand("chat_abort", { sessionId });
}

/** 会话列表（按最后活跃倒序；窗口 B「对话」Tab 左侧列表数据源）。 */
export async function listSessions(): Promise<ChatSessionDto[]> {
  return callCommand<ChatSessionDto[]>("chat_list_sessions");
}

/** 读取指定会话的消息（时间正序；消息区回看用）。 */
export async function getMessages(sessionId: string): Promise<ChatMessageDto[]> {
  return callCommand<ChatMessageDto[]>("chat_get_messages", { sessionId });
}

/** 删除会话（Rust 侧 messages + summaries 同事务清理）。 */
export async function deleteSession(sessionId: string): Promise<void> {
  await callCommand("chat_delete_session", { sessionId });
}

/**
 * memory 命令封装（C7.5 实装）：只读查询 + 写操作（向量同步在后端事务内完成）。
 *
 * 写操作错误语义：`Config` = 目标行不存在 / 当日无对话记录（前端直接呈现）；
 * `Llm` = embed 失败（数据未动，可稍后重试）。
 */

import { callCommand } from "./client";
import type { DiaryDto, FactDto } from "./types";

/** 列出结构化事实。 */
export async function listFacts(): Promise<FactDto[]> {
  return callCommand<FactDto[]>("memory_list_facts");
}

/** 列出日记。 */
export async function listDiaries(): Promise<DiaryDto[]> {
  return callCommand<DiaryDto[]>("memory_list_diaries");
}

/** 编辑事实（含向量同步）——[C7 接入]。 */
export async function updateFact(id: number, content: string): Promise<void> {
  await callCommand("memory_update_fact", { id, content });
}

/** 删除事实（含向量级联）——[C7 接入]。 */
export async function deleteFact(id: number): Promise<void> {
  await callCommand("memory_delete_fact", { id });
}

/** 重新生成指定日期日记（upsert 覆盖）——[C7 接入]。 */
export async function regenerateDiary(date: string): Promise<void> {
  await callCommand("memory_regenerate_diary", { date });
}

/** 新增用户记忆（D3.1 正向入口）：content + category（0=喜好/1=日程/2=档案）。 */
export async function addFact(content: string, category: number): Promise<void> {
  await callCommand("memory_add_fact", { content, category });
}

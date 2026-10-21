/**
 * memory 命令封装：B5 阶段可用的是**只读查询**；写操作签名已定，后端随 C7 接入
 * （当前返回 `AppError::Config("[C7 接入] …")`，前端按错误呈现即可）。
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

/**
 * quota 命令封装（规范 §3：唯一 invoke 出口 = `callCommand`）。
 *
 * 仅内置体验额度：自定义 BYOK 的消耗后端本就不落库、不返回。
 * 周期键为本地自然月（Rust 侧 chrono 口径），前端无需传参。
 */

import { callCommand } from "./client";
import type { QuotaSummary, QuotaUsageRecord } from "./types";

/** 当月汇总（已用/总量/剩余比例/请求次数；空月返回默认发放额度）。 */
export async function getQuotaSummary(): Promise<QuotaSummary> {
  return callCommand<QuotaSummary>("quota_summary");
}

/** 当月明细（倒序；limit 缺省 200，审计/对账用）。 */
export async function listQuotaRecords(limit?: number): Promise<QuotaUsageRecord[]> {
  return callCommand<QuotaUsageRecord[]>("quota_usage_records", { limit });
}

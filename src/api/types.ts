/**
 * IPC DTO 类型真源（规范 §3）：与 Rust serde 结构、接口契约 §2 一一对应。
 *
 * 变更纪律：契约变更先改 `docs/design/接口契约.md`，再同步本文件与 `src-tauri/src/db/models.rs`。
 */

/**
 * 与 src-tauri/src/error.rs 的 AppError 序列化结构对齐（serde tag/content）。
 * 注意：Rust `rename_all = "camelCase"` 对邻接标签枚举的 tag 同样生效，
 * 实际输出为小写 kind（db/llm/window/probe/io/config），unknown 兜底非 Rust 错误。
 */
export type AppErrorKind =
  "db" | "llm" | "window" | "probe" | "io" | "config" | "unknown";

/** 规整后的错误对象（api/client.ts 产出）。 */
export interface AppError {
  kind: AppErrorKind;
  message: string;
}

/** 三档模型配置档位（主对话 / 记忆沉淀 / 兜底）。 */
export type LlmTierName = "main" | "memory" | "fallback";

/** 单档配置视图：Key 绝不明文回传（hasKey + 末 4 位指纹，接口契约 §2）。 */
export interface LlmTierConfig {
  baseUrl: string;
  model: string;
  modelCustom: string;
  hasKey: boolean;
  /** Key 末 4 位；未配置时为 null。 */
  keyFingerprint: string | null;
}

/** 设置快照（settings_get_typed 返回）。 */
export interface SettingsSnapshot {
  main: LlmTierConfig;
  memory: LlmTierConfig;
  fallback: LlmTierConfig;
  /** 1=内置额度 / 2=BYOK（app_settings.is_custom_quota_used）。 */
  isCustomQuotaUsed: number;
  dnd: DndConfig;
}

/** 勿扰配置（勘误#13：多段时段）。 */
export interface DndConfig {
  enabled: boolean;
  ranges: Array<{ start: string; end: string }>;
}

/** 桌宠状态（pet_get_state / pet_update_state 返回）。 */
export interface PetStateDto {
  petName: string;
  relationshipLevel: number;
  affectionPoints: number;
  lastInteraction: string | null;
}

/** 好感度流水条目（pet_list_affection_logs 返回，D2 看板）。 */
export interface LogItem {
  id: number;
  changeValue: number;
  reason: string;
  createdAt: string;
}

/** 主动频率档位（C6 微随机引擎日预算区间：1-2 / 4-6 / 10+）。 */
export type ProactiveFrequency = "quiet" | "normal" | "sticky";

/** 聊天气泡状态（chatStore，窗口 A）。 */
export interface ChatBubbleState {
  sessionId: string;
  text: string;
  isStreaming: boolean;
}

/** 事实条目（memory_list_facts 返回）。 */
export interface FactDto {
  id: number;
  category: number;
  content: string;
  confidence: number;
  createdAt: string;
}

/** 日记条目（memory_list_diaries 返回）。 */
export interface DiaryDto {
  id: number;
  diaryDate: string;
  content: string;
  mood: number | null;
}

/** 保存单档配置的入参（api_key 仅在新值时携带；空串 = 不改动存量 Key）。 */
export interface SaveTierPayload {
  baseUrl: string;
  model: string;
  apiKey?: string;
  modelCustom: string;
}

/** 更新桌宠资料的入参（好感度字段不接受修改——结算属 C5）。 */
export interface UpdatePetPayload {
  petName?: string;
  activeRenderer?: string;
  rendererConfig?: unknown;
}

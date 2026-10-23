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
  /** 主动频率档位（D2.3：随 type=2 行下发；C6 引擎读取）。 */
  frequency: ProactiveFrequency;
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

/** 好感度流水 DTO（Rust `AffectionLogDto` 对齐，camelCase）。 */
export interface AffectionLogDto {
  id: number;
  changeValue: number;
  reason: string;
  createdAt: string;
}

/** 好感度结算信号入参（前端仅发 checkIn / petStroke；deepChat/resonance 由 C4 内部调用）。 */
export type AffectionSettleInput =
  | { kind: "checkIn" }
  | { kind: "petStroke" }
  | { kind: "deepChat" }
  | { kind: "resonance"; strength: number }
  | { kind: "taboo"; value: number };

/** affection:changed 事件负载（窗口 A 只消费 mood；delta/newPoints/newLevel 仅进看板 D2）。 */
export interface AffectionChangedPayload {
  delta: number;
  newPoints: number;
  newLevel: number;
  mood: "happy" | "angry" | "neutral";
}

/** 主动频率档位（C6 微随机引擎日预算区间：1-2 / 4-6 / 10+）。 */
export type ProactiveFrequency = "quiet" | "normal" | "sticky";

/** 聊天气泡状态（chatStore，窗口 A）。 */
export interface ChatBubbleState {
  sessionId: string;
  text: string;
  isStreaming: boolean;
}

/** 会话列表项（chat_list_sessions 返回；与 Rust `SessionSummaryDto` 对齐）。 */
export interface ChatSessionDto {
  sessionId: string;
  /** 标题预览：首条 user 消息；空串表示尚未产生用户消息（UI 显示"新对话"）。 */
  preview: string;
  /** 最后活跃时间（UTC 字符串，Rust CURRENT_TIMESTAMP）。 */
  lastAt: string;
  messageCount: number;
}

/** 会话消息（chat_get_messages 返回；role: 0=user / 1=assistant）。 */
export interface ChatMessageDto {
  id: number;
  role: number;
  content: string;
}

/** 事实条目（memory_list_facts 返回）。 */
export interface FactDto {
  id: number;
  category: number;
  content: string;
  confidence: number;
  createdAt: string;
}

/** 日记情绪档（C7：LLM 同步输出；0=开心 1=平静 2=低落；越界/未知 → null）。 */
export type DiaryMood = 0 | 1 | 2;

/** 日记条目（memory_list_diaries 返回）。 */
export interface DiaryDto {
  id: number;
  diaryDate: string;
  content: string;
  mood: DiaryMood | null;
}

/** 保存单档配置的入参（api_key 仅在新值时携带；空串 = 不改动存量 Key）。 */
export interface SaveTierPayload {
  baseUrl: string;
  model: string;
  apiKey?: string;
  modelCustom: string;
}

/** 服务商预设（C3：来自 Rust `config::PROVIDER_PRESETS`，经 IPC 下发——前端不复制列表）。 */
export interface ProviderPreset {
  id: string;
  label: string;
  baseUrl: string;
  defaultModel: string;
  required: boolean;
  /** 成本档（展示用，不写死价格）。 */
  costTier: string;
}

/** LLM 配置档位（C3 常量；与 `TierName` 白名单一致）。 */
export type TierName = "main" | "memory" | "fallback";

/** 更新桌宠资料的入参（好感度字段不接受修改——结算属 C5）。 */
export interface UpdatePetPayload {
  petName?: string;
  activeRenderer?: string;
  rendererConfig?: unknown;
}

/** 命中区域（窗口局部坐标，逻辑像素；与 Rust `state::HitRegion` 对齐）。 */
export interface HitRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** 鼠标坐标事件负载（probe:cursor:move，屏幕坐标）。 */
export interface CursorPayload {
  x: number;
  y: number;
}

/** 空闲状态事件负载（probe:idle:changed；C6 翻转才 emit）。 */
export interface IdlePayload {
  state: "idle" | "active";
  idleMs: number;
}

/** 前台进程事件负载（probe:foreground:changed；仅进程名，变化才 emit）。 */
export interface ForegroundPayload {
  process: string | null;
}

/** 探针快照（probe_get_snapshot；非订阅场景读缓存）。 */
export interface ProbeSnapshot {
  cursor: [number, number];
  idleMs: number;
  idle: boolean;
  foreground: string | null;
}

/** 拖入/附件负载（D1.5；与 Rust `FilePayload` 对齐）。 */
export interface FilePayload {
  /** 展示名（文件名）。 */
  name: string;
  /** "text" | "image"（Rust 侧再次校验）。 */
  kind: "text" | "image";
  /** text=utf8 内容；image=base64 data URL（含 `data:...;base64,` 前缀）。 */
  content: string;
  /** image/png 等 MIME。 */
  mime: string;
}

/** proactive:show 事件负载（Rust 非流式短生成完毕；前端仅做本地打字机）。 */
export interface ProactiveShowPayload {
  sessionId: string;
  text: string;
  /** 表情指令（与 PetExpression 值域一致；C7 禁忌补充 angry）。 */
  mood: "happy" | "tsundere" | "angry";
}

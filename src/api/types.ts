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

// ---------- 全局划词翻译（接口契约 §2 translate_*） ----------

/** 本版仅中英互译；auto = 按源语言自动取相反语言。 */
export type TranslateTargetLang = "auto" | "zh" | "en";
export type TranslateProviderId = "baidu" | "deepl" | "youdao" | "google";

/** 划词翻译配置（Key 脱敏，接口不回明文）。 */
export interface TranslateSettings {
  enabled: boolean;
  targetLang: TranslateTargetLang;
  provider: TranslateProviderId;
  hasBuiltinCredential: boolean;
  hasCustomKey: boolean;
  customKeyFingerprint: string | null;
}

/** 保存配置入参；Key 缺省=不改动，空串=清空。 */
export interface SaveTranslateSettings {
  enabled: boolean;
  targetLang: TranslateTargetLang;
  provider: TranslateProviderId;
  customApiKey?: string;
  customAppId?: string;
}

/** 选区屏幕矩形（逻辑像素）。 */
export interface SelectionRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** 取词结果（用户点击工具条后返回）。 */
export interface CapturedSelection {
  selectionKey: string;
  text: string;
  rect: SelectionRect;
  via: "uia" | "clipboard";
}

/** 译文结果（浮窗展示 + 重新翻译复用）。 */
export interface TranslateResult {
  resultId: string;
  sourceText: string;
  translatedText: string;
  sourceLang: "zh" | "en";
  targetLang: "zh" | "en";
  /** 内置专属标识 `xfyun` 不可自填，但可能作为实际使用的服务商回传。 */
  provider: TranslateProviderId | "xfyun";
  /** 实际使用的服务名（自填展示名 / 内置「百度翻译」「讯飞翻译」）。 */
  serviceName: string;
  isBuiltin: boolean;
  quotaExhausted: boolean;
}

/** `translate:selection:show` 事件负载。 */
export interface TranslateSelectionPayload {
  selectionKey: string;
  rect: SelectionRect;
  cursorX: number;
  cursorY: number;
}

/** 用户自填的翻译服务商（Key 脱敏，接口不回明文）。 */
export interface CustomTranslateProvider {
  id: number;
  name: string;
  provider: TranslateProviderId;
  hasAppId: boolean;
  hasKey: boolean;
  keyFingerprint: string | null;
  enabled: boolean;
}

/** 保存自填服务商入参；apiKey 缺省=不改动、空串=清空。 */
export interface SaveCustomTranslateProvider {
  id?: number;
  name: string;
  provider: TranslateProviderId;
  /** 缺省=不改动存量；空串=清空。 */
  appId?: string;
  apiKey?: string;
  enabled?: boolean;
}

/** 内置翻译服务商（百度 / 讯飞）：凭据脱敏，只回指纹与当月用量。 */
export interface BuiltinTranslateProvider {
  id: number;
  providerId: string;
  name: string;
  enabled: boolean;
  monthlyGranted: number;
  used: number;
  requests: number;
  hasCredential: boolean;
  credentialFingerprint: string | null;
}

/** 内置服务商配置由底层固定，前端只读其用量视图，不提供写入入参。 */

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
  /** 边缘吸附休眠开关（type=3 行；缺省 true——拖到屏幕边缘即休眠）。 */
  snapSleepEnabled: boolean;
  /** 吸附态露出比例（type=3 行；占形象圆盘直径的比例，缺省 0.5）。 */
  snapShowRatio: number;
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
  /** 当前生效的渲染器 id（`renderer/factory.ts` 注册表键，如 `code_cat` / `code_dog`）。 */
  activeRenderer: string;
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

/** 自定义模型视图（settings_list_custom_models 返回；Key 脱敏口径同 LlmTierConfig）。 */
export interface CustomModelDto {
  id: number;
  name: string;
  baseUrl: string;
  /** 模型名列表（编排时按序展开）。 */
  models: string[];
  hasKey: boolean;
  keyFingerprint: string | null;
  /** 是否启用（停用不删，保配置）。 */
  enabled: boolean;
}

/** 保存自定义模型入参（id 缺省=新建；apiKey 缺省=不改动，空串=清空）。 */
export interface SaveCustomModelPayload {
  id?: number;
  name: string;
  baseUrl: string;
  models: string[];
  apiKey?: string;
  enabled?: boolean;
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

/** 保存单档配置的入参（api_key 仅在新值时携带；空串 = 不改动存量 Key）。
 *  注：三档 UI 已合并为「自定义大模型服务商」平台列表（2026-10-24），
 *  保存入口随之移除；类型保留仅供存量快照读取兼容。 */
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
  /** 常见模型列表（面板「+ 快速添加」数据源）。 */
  defaultModels: string[];
  required: boolean;
  /** 成本档（展示用，不写死价格）。 */
  costTier: string;
}

/** 内置额度月度汇总（quota_summary 返回；界面展示用，Rust `QuotaSummaryDto` 对齐）。 */
export interface QuotaSummary {
  /** 本地自然月周期键 YYYY-MM。 */
  periodKey: string;
  /** 本月发放额度（credits）。 */
  granted: number;
  /** 本月已用额度（credits）。 */
  used: number;
  /** 剩余比例 0–1（进度条直接可用）。 */
  remainingRatio: number;
  /** 本月记账请求次数。 */
  requests: number;
  /** 按平台拆分的当月用量（used 降序；空月为空数组）。 */
  platforms: QuotaPlatformUsage[];
}

/** 单个平台的当月内置额度用量（仅内置段记账）。 */
export interface QuotaPlatformUsage {
  /** 平台名（default_model_settings.name，如 智谱 / 硅基流动；无归属为 unknown）。 */
  platform: string;
  /** 该平台当月已用 credits。 */
  used: number;
  /** 该平台当月记账请求次数。 */
  requests: number;
}

/** 内置额度单次请求明细（quota_usage_records 返回，审计/对账用）。 */
export interface QuotaUsageRecord {
  id: number;
  createdAt: string;
  periodKey: string;
  /** 场景：chat 主对话 / proactive 主动话术 / distill 蒸馏 / diary 日记。 */
  scene: string;
  /** 命中链段：builtin 内置额度 / custom 自定义 / main / memory / fallback。 */
  tier: string;
  /** 模型所属平台名（内置 = 智谱 / 硅基流动；无归属为 unknown）。 */
  platform: string;
  model: string;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  /** 计权后扣减点数（失败为 0）。 */
  credits: number;
  ok: boolean;
  /** 失败原因（成功为 null）。 */
  error: string | null;
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

/** 联网搜索服务配置（search_settings_get 返回；Key 脱敏，接口契约 §2）。 */
export interface SearchSettings {
  /** 服务商：tavily / brave / serpapi。 */
  provider: string;
  /** 是否已配置 Key（用于 UI 提示「未配置则回落普通对话」）。 */
  hasKey: boolean;
  /** Key 末 4 位指纹；未配置为 null。 */
  keyFingerprint: string | null;
  /** 是否启用联网搜索。 */
  enabled: boolean;
  /** 单次搜索结果上限。 */
  maxResults: number;
  /** 结果语言偏好：zh / en。 */
  lang: string;
}

/** 保存联网搜索配置（apiKey 缺省=不改动存量 Key，空串=清空）。 */
export interface SaveSearchSettings {
  provider: string;
  apiKey?: string;
  enabled: boolean;
  maxResults: number;
  lang: string;
}

/** 联网搜索引用来源（chat:stream:sources 事件负载；气泡底部渲染）。 */
export interface ChatSource {
  title: string;
  url: string;
}

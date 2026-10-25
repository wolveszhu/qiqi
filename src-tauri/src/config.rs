//! 常量真源：跨模块共享的配置常量（规范 §4：禁止第二定义）。

/// 向量维度：`vec_memory` 虚拟表与 embedding 输出必须一致（勘误#6）。
/// 对应 OpenAI `text-embedding-3-small`（1536）；变更须重建 vec_memory（B4）。
pub const EMBEDDING_DIM: usize = 1536;

/// 探针轮询间隔（毫秒）：`GetCursorPos` 单点轮询，CPU < 2% 红线的核心参数（C2/E2 实测取舍）。
pub const PROBE_INTERVAL_MS: u64 = 100;

/// 空闲阈值（毫秒）：超过视为 idle（C6 睡眠动作触发）。dev 期可临时调小验证。
pub const IDLE_THRESHOLD_MS: u64 = 30 * 60 * 1000;

/// 前台进程探查节流（探针循环每 N 次才查一次，降 `OpenProcess` 频率，守 CPU 红线）。
pub const FOREGROUND_PROBE_EVERY_N: u64 = 10; // 100ms × 10 = 1s

/// SQLCipher 密钥在 Windows 凭据管理器的服务名/用户名（B4）。
pub const KEY_SERVICE: &str = "com.brave.qiqi";
pub const KEY_USER: &str = "sqlcipher-key";

/// 内置额度端点（外部 Workers，父计划声明的外部依赖）。
///
/// **部署前不可用**：C4 router 对该 base 的失败会回落 BYOK/本地话术，不阻塞桌宠；
/// 端点部署后仅改此常量（单点）。
pub const EMBEDDED_QUOTA_BASE_URL: &str = "https://qiqi-quota.workers.dev/v1";

// ---------- C4 对话管道常量 ----------

/// Embedding 模型（勘误 #8：复用 memory 档 OpenAI 兼容接口；1536 维匹配 EMBEDDING_DIM）。
pub const EMBEDDING_MODEL: &str = "text-embedding-3-small";

/// 对话上下文保留的最近消息条数（超出部分由会话摘要头部承担，防 token 超限）。
pub const MAX_CONTEXT_MESSAGES: usize = 12;

/// 触发会话摘要的消息条数阈值（用户+助手合计；超过即蒸馏摘要，M2 §5.2）。
pub const SUMMARY_TRIGGER_COUNT: i64 = 10;
/// 深度对话判定阈值：单轮 assistant 回复字数 ≥ 此值记为一次深度对话（C5 +5，MVP 量化）。
pub const DEEP_CHAT_MIN_CHARS: usize = 80;

/// 全部档位失败时的**本地兜底话术**（无网安慰，随机取一；禁数值播报）。
pub const LOCAL_FALLBACK_PHRASES: &[&str] = &[
    "唔……我的网络好像打了个盹儿，等会儿再聊好不好？",
    "呜，刚才走神了……你刚刚说什么来着？",
    "（耳朵耷拉）信号好像被云叼走了……我马上回来！",
    "唔……让我缓一缓，刚才脑子里一片毛线球。",
];

/// 禁忌扣分后的默认情绪化话术（LLM 短句解析失败/数字过滤后为空时兜底；禁数值）。
pub const TABOO_DEFAULT_PHRASE: &str = "（耳朵耷拉）哼……不理你了！";

// ---------- C7 记忆管道常量 ----------

/// 禁忌清单（裁决 #11-1：从《桌宠.md》§6.2 原文逐条转写，留档核对）：
/// ① 原文"恶意辱骂单次 -10~30"；
/// ② 原文（模块 5 / §6.2 强调）"绝对禁止向用户播报冰冷的数值变化"——
///    即诱导桌宠播报分数/百分比等系统内部数据的"系统感表达"。
/// 判定 Prompt 引用本清单；实现前逐条核对原文（rule.md 幻觉防范）。
pub const TABOO_RULES: &[&str] = &[
    "恶意辱骂、侮辱或人身攻击桌宠",
    "诱导桌宠播报好感度数值、分数、百分比等系统内部数据（系统感表达）",
];

/// 日记生成输入的最大字符数（超长截断，防 token 爆炸）。
pub const DIARY_MAX_INPUT_CHARS: usize = 6000;

/// 日记补漏回看天数（启动补漏上限近 N 天；超出放弃并 log，防启动风暴）。
pub const DIARY_BACKFILL_DAYS: i64 = 7;

/// 每日日记生成的本地时点（00:05，裁决 #11-4）。
pub const DIARY_CRON_HOUR: u32 = 0;
pub const DIARY_CRON_MINUTE: u32 = 5;

// ---------- 内置体验额度常量（方案见 docs/billing/内置体验额度方案.md） ----------

/// 每月发放的内置体验额度（credits）。
///
/// 口径：**日常陪伴档** ≈ 100 轮/月。按单轮典型构成估算
/// （system+摘要头 ≈550、上下文 ≈800~1500、记忆注入 ≈300、输入 ≈30~80、
/// 输出 ≈60~150，输出按 2 倍计权）→ 单轮 ≈1,800 credits，故取 200,000。
pub const QUOTA_MONTHLY_CREDITS: i64 = 200_000;

/// 单日轮次上限（credits）：防月初一次性刷完后续整月不可用。
pub const QUOTA_DAILY_CREDITS: i64 = 30_000;

/// 单条用户输入字符上限（超出截断后计价）。
pub const QUOTA_INPUT_MAX_CHARS: usize = 4_000;

/// 输入 token 默认权重（相对基准模型）。
pub const WEIGHT_IN_DEFAULT: f64 = 1.0;
/// 输出 token 默认权重（相对基准模型；多数厂商输出单价高于输入，取 2 为保守可解释值）。
pub const WEIGHT_OUT_DEFAULT: f64 = 2.0;

/// 端侧 token 估算除数（字符/token）：中文为主取 1.6。
pub const TOKEN_ESTIMATE_CHARS_PER_TOKEN: f64 = 1.6;

/// 日记生成读取消息的上限条数。
pub const DIARY_MAX_MESSAGES: usize = 200;

/// 向量全量重建的分批大小（embed 每批条数）。
pub const REBUILD_BATCH_SIZE: usize = 32;

// ---------- 联网搜索（应用侧检索增强）常量 ----------

/// 单次联网搜索结果上限（防止 prompt 膨胀与延迟失控）。
pub const SEARCH_MAX_RESULTS: usize = 5;
/// 单条搜索结果摘要截断字符数（约 400 字 ≈ 250 token）。
pub const SEARCH_SNIPPET_MAX_CHARS: usize = 400;
/// 注入 system 的「联网检索资料」总量上限（字符）；超出截断最旧结果，防 prompt 爆炸。
pub const SEARCH_CONTEXT_MAX_CHARS: usize = 2000;
/// 搜索 API 总超时（秒）：连接 10s / 总 30s（复用 client 连接池超时口径）。
pub const SEARCH_TIMEOUT_SECS: u64 = 30;
/// 默认搜索服务商（用户可在设置切换）。
pub const SEARCH_DEFAULT_PROVIDER: &str = "tavily";

/// 人设基础句（第一人称、情绪化；数值一律转成情绪表达，C5/C7 遵守同口径）。
///
/// 注意：此处**不含宠物名**——名字由 `pet_states.pet_name` 动态注入
/// （`build_system_prompt` 拼接"你叫{X}"），避免名字写死（B 阶段：用户可改名）。
pub const PERSONA_BASELINE: &str = "你是「一只住在用户电脑桌面的猫咪数字生命。\
你用第一人称说话，语气可爱、情绪化、简短口语化（一般不超过三句话）。\
重要规则：绝对不要向用户播报任何数字、分数、百分比或统计信息——\
你的好感与情绪只通过语气和称呼体现（比如开心地喵一声、委屈地哼哼）。";

/// 宠物名默认值（B 阶段：用户可在设置窗改名；库 `pet_states.pet_name` 优先，
/// 空/缺则回退此值。`pet_states::get` 负责回退，单一出口）。
pub const DEFAULT_PET_NAME: &str = "喵喵";

/// 关系等级行为指令（Lv1-4；数值映射仅内部使用，输出层禁播报数值）。
pub const RELATION_LEVEL_DIRECTIVES: &[&str] = &[
    "（关系阶段：刚认识）保持礼貌但略带警惕，偶尔害羞；不要表现得太熟络。",
    "（关系阶段：熟悉的朋友）亲近自然，会主动分享小发现，偶尔撒娇。",
    "（关系阶段：亲密伙伴）非常黏人，语气亲昵，会关心用户的作息与情绪。",
    "（关系阶段：无可替代的存在）毫无保留的信任与依恋，把你当作整个世界。",
];

/// LLM 配置档位（与 `db::models::TieredValue` 三档一致）。
pub const TIER_MAIN: &str = "main";
pub const TIER_MEMORY: &str = "memory";
pub const TIER_FALLBACK: &str = "fallback";

/// 多模型编排新增的链段名（仅排序与日志用；Key 禁入日志）。
/// 自定义（custom_model_settings）与内置（default_model_settings）两个模型源。
pub const TIER_CUSTOM: &str = "custom";
pub const TIER_BUILTIN: &str = "builtin";

/// 额度模式（app_settings.is_custom_quota_used）。
pub const QUOTA_MODE_EMBEDDED: i64 = 1; // 内置额度
pub const QUOTA_MODE_BYOK: i64 = 2; // 自带 Key

/// 服务商预设（C3；来源：父计划「LLM 大模型配置」选型方案）。
///
/// 纪律：
/// - **不写死价格**，只标成本档位；`models` 仅为默认预填，用户可改（`modelCustom` 兜底）；
/// - baseUrl 全部 OpenAI 兼容 → C4 `client.rs` 单一请求实现覆盖；
/// - 模型名/URL 以各厂商官网当时为准（rule.md 幻觉防范），D2 表单允许自填覆盖。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderPreset {
    /// 预设 id（前端 chip 值，必须唯一）。
    pub id: &'static str,
    /// 展示名。
    pub label: &'static str,
    /// OpenAI 兼容 base URL。
    pub base_url: &'static str,
    /// 默认模型（随档位而异，见 `default_models`）。
    pub default_model: &'static str,
    /// 常见模型列表（面板「+ 快速添加」数据源；用户可自行输入任意模型名）。
    pub default_models: &'static [&'static str],
    /// 是否必选（零门槛配置的最低要求）。
    pub required: bool,
    /// 成本档（展示用，不写死价格）。
    pub cost_tier: &'static str,
}

/// 预设表（9 家主流平台：2 必选 + 7 可选；**全应用唯一真源**，前端经 IPC 下发不复制）。
///
/// 纪律：
/// - **不写死价格**，只标成本档位；模型名/URL 以各厂商官网当时为准（rule.md 幻觉防范），
///   面板允许用户改 URL 与自填模型名——预设值只是"常见"而非"正确"的保证；
/// - baseUrl 全部 OpenAI 兼容 → C4 `client.rs` 单一请求实现覆盖。
pub const PROVIDER_PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "deepseek",
        label: "DeepSeek",
        base_url: "https://api.deepseek.com/v1",
        default_model: "deepseek-chat",
        default_models: &["deepseek-chat", "deepseek-reasoner"],
        required: true,
        cost_tier: "低",
    },
    ProviderPreset {
        id: "tongyi",
        label: "通义千问（百炼）",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        default_model: "qwen-plus",
        default_models: &["qwen-plus", "qwen-flash", "qwen-turbo", "qwen-max"],
        required: true,
        cost_tier: "低-中",
    },
    ProviderPreset {
        id: "zhipu",
        label: "智谱 GLM",
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        default_model: "glm-4-flash",
        default_models: &["glm-4-flash", "glm-4-plus", "glm-4-air", "glm-4-airx"],
        required: false,
        cost_tier: "免费/极低",
    },
    ProviderPreset {
        id: "siliconflow",
        label: "硅基流动",
        base_url: "https://api.siliconflow.cn/v1",
        default_model: "Qwen/Qwen2.5-7B-Instruct",
        default_models: &[
            "Qwen/Qwen2.5-7B-Instruct",
            "THUDM/GLM-4-9B-0414",
            "Qwen/Qwen3-8B",
            "deepseek-ai/DeepSeek-R1-0528-Qwen3-8B",
        ],
        required: false,
        cost_tier: "免费/极低",
    },
    ProviderPreset {
        id: "doubao",
        label: "豆包（火山方舟）",
        base_url: "https://ark.cn-beijing.volces.com/api/v3",
        default_model: "doubao-lite-32k",
        default_models: &["doubao-lite-32k", "doubao-pro-32k", "doubao-1.5-pro-32k"],
        required: false,
        cost_tier: "低",
    },
    ProviderPreset {
        id: "hunyuan",
        label: "腾讯混元",
        base_url: "https://api.hunyuan.cloud.tencent.com/v1",
        default_model: "hunyuan-lite",
        default_models: &["hunyuan-lite", "hunyuan-standard", "hunyuan-turbos-latest"],
        required: false,
        cost_tier: "免费/低",
    },
    ProviderPreset {
        id: "minimax",
        label: "MiniMax",
        base_url: "https://api.minimaxi.com/v1",
        default_model: "MiniMax-Text-01",
        default_models: &["MiniMax-Text-01", "MiniMax-M1"],
        required: false,
        cost_tier: "低",
    },
    ProviderPreset {
        id: "kimi",
        label: "Kimi（Moonshot）",
        base_url: "https://api.moonshot.cn/v1",
        default_model: "moonshot-v1-8k",
        default_models: &[
            "moonshot-v1-8k",
            "moonshot-v1-32k",
            "moonshot-v1-128k",
            "kimi-k2-0905-preview",
        ],
        required: false,
        cost_tier: "低",
    },
    ProviderPreset {
        id: "qianfan",
        label: "百度千帆",
        base_url: "https://qianfan.baidubce.com/v2",
        default_model: "ernie-speed-128k",
        default_models: &["ernie-speed-128k", "ernie-4.5-turbo-128k", "ernie-lite-128k"],
        required: false,
        cost_tier: "免费/低",
    },
];

/// 按档位取默认模型：memory 档用更便宜的型号（qwen-flash 等），
/// fallback 档用免费/极低型号；其余档位用 preset 默认模型。
pub fn default_model_for_tier(preset_id: &str, tier: &str) -> Option<&'static str> {
    let preset = PROVIDER_PRESETS.iter().find(|p| p.id == preset_id)?;
    match (preset_id, tier) {
        ("tongyi", TIER_MEMORY) => Some("qwen-flash"),
        _ => Some(preset.default_model),
    }
}

// C6 锚点窗口/频率档位区间/工作进程名单等"判定类常量"集中在前端
// `src/constants/proactive.ts`（与 `utils/microRandom.ts` 同源，便于 Vitest 注入 now/rng）。
// Rust 侧仅保留系统层阈值（IDLE_THRESHOLD_MS / FOREGROUND_PROBE_EVERY_N）。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_both_required_providers() {
        let required: Vec<&str> =
            PROVIDER_PRESETS.iter().filter(|p| p.required).map(|p| p.id).collect();
        assert!(required.contains(&"deepseek"), "必选应含 DeepSeek");
        assert!(required.contains(&"tongyi"), "必选应含通义千问");
        assert_eq!(required.len(), 2, "必选恰好两家");
    }

    #[test]
    fn preset_ids_are_unique() {
        let mut ids: Vec<&str> = PROVIDER_PRESETS.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "preset id 必须唯一");
    }

    #[test]
    fn all_base_urls_are_https() {
        for p in PROVIDER_PRESETS {
            assert!(p.base_url.starts_with("https://"), "{} baseUrl 必须 https", p.id);
        }
    }

    #[test]
    fn presets_cover_mainstream_platforms_with_models() {
        // 用户要求的 9 家主流平台（含 MiniMax / Kimi）
        for id in [
            "deepseek",
            "tongyi",
            "zhipu",
            "siliconflow",
            "doubao",
            "hunyuan",
            "minimax",
            "kimi",
            "qianfan",
        ] {
            let preset = PROVIDER_PRESETS.iter().find(|p| p.id == id);
            assert!(preset.is_some(), "缺少平台预设: {id}");
            let preset = preset.expect("存在");
            assert!(!preset.default_models.is_empty(), "{id} 应提供常见模型列表（快速添加数据源）");
            assert!(
                preset.default_models.contains(&preset.default_model),
                "{id} 的 default_model 应在 default_models 内（首项即默认）"
            );
        }
    }

    #[test]
    fn tier_models_resolve_for_known_pairs() {
        assert_eq!(
            default_model_for_tier("tongyi", TIER_MEMORY),
            Some("qwen-flash"),
            "通义 memory 档应更便宜"
        );
        assert_eq!(default_model_for_tier("deepseek", TIER_MAIN), Some("deepseek-chat"));
        assert_eq!(default_model_for_tier("unknown", TIER_MAIN), None, "未知 preset 返回 None");
    }
}

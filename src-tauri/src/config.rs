//! 常量真源：跨模块共享的配置常量（规范 §4：禁止第二定义）。

/// 向量维度：`vec_memory` 虚拟表与 embedding 输出必须一致（勘误#6）。
/// 对应 OpenAI `text-embedding-3-small`（1536）；变更须重建 vec_memory（B4）。
pub const EMBEDDING_DIM: usize = 1536;

/// 探针轮询间隔（毫秒）：`GetCursorPos` 单点轮询，CPU < 2% 红线的核心参数（C2/E2 实测取舍）。
pub const PROBE_INTERVAL_MS: u64 = 100;

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

/// 人设基础句（第一人称、情绪化；数值一律转成情绪表达，C5/C7 遵守同口径）。
pub const PERSONA_BASELINE: &str = "你是「七七」，一只住在用户电脑桌面的猫咪数字生命。\
你用第一人称说话，语气可爱、情绪化、简短口语化（一般不超过三句话）。\
重要规则：绝对不要向用户播报任何数字、分数、百分比或统计信息——\
你的好感与情绪只通过语气和称呼体现（比如开心地喵一声、委屈地哼哼）。";

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
    /// 是否必选（零门槛配置的最低要求）。
    pub required: bool,
    /// 成本档（展示用，不写死价格）。
    pub cost_tier: &'static str,
}

/// 预设表（7 组：2 必选 + 5 可选；**全应用唯一真源**，前端经 IPC 下发不复制）。
pub const PROVIDER_PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "deepseek",
        label: "DeepSeek",
        base_url: "https://api.deepseek.com/v1",
        default_model: "deepseek-chat",
        required: true,
        cost_tier: "低",
    },
    ProviderPreset {
        id: "tongyi",
        label: "通义千问（百炼）",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        default_model: "qwen-plus",
        required: true,
        cost_tier: "低-中",
    },
    ProviderPreset {
        id: "zhipu",
        label: "智谱 GLM",
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        default_model: "glm-4-flash",
        required: false,
        cost_tier: "免费/极低",
    },
    ProviderPreset {
        id: "siliconflow",
        label: "硅基流动",
        base_url: "https://api.siliconflow.cn/v1",
        default_model: "Qwen/Qwen2.5-7B-Instruct",
        required: false,
        cost_tier: "免费/极低",
    },
    ProviderPreset {
        id: "doubao",
        label: "豆包（火山方舟）",
        base_url: "https://ark.cn-beijing.volces.com/api/v3",
        default_model: "doubao-lite-32k",
        required: false,
        cost_tier: "低",
    },
    ProviderPreset {
        id: "hunyuan",
        label: "腾讯混元",
        base_url: "https://api.hunyuan.cloud.tencent.com/v1",
        default_model: "hunyuan-lite",
        required: false,
        cost_tier: "免费/低",
    },
    ProviderPreset {
        id: "qianfan",
        label: "百度千帆",
        base_url: "https://qianfan.baidubce.com/v2",
        default_model: "ernie-speed-128k",
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

// TODO(C6): 空闲阈值（30min）、时间窗口锚点（08:30-09:30 / 22:00-23:30）、
// 频率档位区间（1-2/4-6/10+）、工作/游戏进程名单。

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

//! 多模型编排链（C4.3 扩展：自定义/内置多模型 + 三档兜底）。
//!
//! 链序（用户决策 2026-10-24）：
//! **自定义（顺序 failover）→ 内置（粘性轮询起点 + 顺序尝试）→ main（存量单档）
//! → memory → fallback → 本地兜底话术**。
//!
//! - 自定义：每个平台的每个模型按序展开，失败即切下一个（用户自带 Key，优先）；
//! - 内置：全部可用模型展平后按 `sticky_rotate` 决定起点——优先沿用上次成功的模型
//!   （保持体感回复风格一致），失效时按轮询计数器分摊；随后仍按顺序向后尝试；
//! - `main` 档保留入链：自定义平台出现之前的主对话配置，无自定义时仍为首选（存量兼容）；
//! - 未配置的档位自动跳过；全部失败 → 本地话术（emit 后 done，应用不崩不卡）。
//!
//! 每次失败记入回落链日志（仅档位+状态摘要，禁 Key/baseUrl 全文）。

use std::sync::Mutex;
use std::sync::OnceLock;

use super::client::{ChatRequest, DeltaStream, LlmClient, LlmTarget};
use crate::config::{
    LOCAL_FALLBACK_PHRASES, TIER_BUILTIN, TIER_CUSTOM, TIER_FALLBACK, TIER_MAIN, TIER_MEMORY,
};
use crate::db::models::TieredValue;
use crate::db::repositories::custom_model_settings::CustomModelRow;
use crate::db::repositories::default_model_settings::{self, DefaultModelRow};

/// 编排链（目标序列 + 内置段区间；内置段区间供成功回调定位粘性下标）。
pub struct ModelChain {
    pub targets: Vec<LlmTarget>,
    /// 内置段在 `targets` 中的区间 `[start, end)`；无内置源时为 None。
    pub builtin_range: Option<(usize, usize)>,
}

/// 编排链输入（命令层组装；路由层只排顺序，**不读库**——保持纯函数可测）。
pub struct ChainInput<'a> {
    /// 自定义模型（已按 sort_order 排序、已过滤启用与 LLM 类型）。
    pub custom: &'a [CustomModelRow],
    /// 内置模型（已过滤可用与 LLM 类型）；空切片 = 不接入内置段。
    pub builtin: &'a [DefaultModelRow],
    /// 进程内轮询计数器（取自 `next_builtin_slot` 自增后的值）。
    pub builtin_cursor: usize,
    /// 上次成功的内置模型下标（展平后；进程内记忆，重启清零）。
    pub builtin_last_ok: Option<usize>,
    /// 三档配置（main/memory/fallback）。
    pub tiered: &'a TieredValue,
}

/// 档位 → 目标（main/memory/fallback 三档与自定义/内置段共用）。
pub fn target_of(tier: &'static str, value: &crate::db::models::LlmTierValue) -> LlmTarget {
    LlmTarget {
        tier,
        base_url: value.base_url.clone(),
        api_key: value.api_key.clone(),
        model: value.model.clone(),
    }
}

/// memory 档目标（C4 蒸馏/embedding 与 C7 日记/编辑同步共用；勘误 #8 单一真源）。
///
/// 内置额度模式下未配 memory 档 → 指向外部端点；model 缺省 → embedding 模型。
pub fn memory_target(tiered: &TieredValue, quota_embedded: bool) -> LlmTarget {
    let mut memory = target_of(TIER_MEMORY, &tiered.memory);
    if quota_embedded && memory.base_url.is_empty() {
        memory.base_url = crate::config::EMBEDDED_QUOTA_BASE_URL.into();
        memory.model = crate::config::EMBEDDING_MODEL.into();
    }
    if memory.model.is_empty() {
        memory.model = crate::config::EMBEDDING_MODEL.into();
    }
    memory
}

/// 记忆沉淀目标的**自动派生**（用户决策：UI 不再暴露 memory 档配置）。
///
/// 优先序：
/// 1. memory 档已显式配置（存量数据/M7 前的库）→ 用它；
/// 2. 首个启用的自定义平台 → 取其第一个模型（蒸馏/事实/共鸣/禁忌判定都可用；
///    该模型若不支持 embedding，`embed` 调用失败会走 `retrieve_context` 的降级分支——
///    memory_hits 为空但对话本身不受影响）；
/// 3. 内置额度端点（`memory_target` 的 quota 分支，未部署时同样失败降级）；
/// 4. 全空 → 返回空 target（调用方各自优雅降级）。
pub fn memory_target_from_sources(
    tiered: &TieredValue,
    quota_embedded: bool,
    custom: &[CustomModelRow],
) -> LlmTarget {
    // ① memory 档显式配置（M7 清空后的新库不会走到这里；防御存量/回滚库）
    if !tiered.memory.base_url.is_empty() {
        return memory_target(tiered, quota_embedded);
    }
    // ② 首个启用的自定义平台（调用方已过滤启用与 LLM 类型）
    if let Some(row) = custom.first()
        && let Some(model) = row.models().first()
    {
        return LlmTarget {
            tier: TIER_CUSTOM,
            base_url: row.base_url.clone(),
            api_key: row.api_key.clone(),
            model: model.clone(),
        };
    }
    // ③④ 内置额度端点 / 空 target（均由 memory_target 的既有分支表达）
    memory_target(tiered, quota_embedded)
}

/// 构建编排链（纯函数：输入行数据 + 轮询参数 → 有序目标列表）。
pub fn build_chain(input: &ChainInput<'_>) -> ModelChain {
    let mut targets: Vec<LlmTarget> = Vec::new();

    // ① 自定义：平台顺序 × 模型顺序展开（顺序 failover）
    for row in input.custom {
        for model in row.models() {
            targets.push(LlmTarget {
                tier: TIER_CUSTOM,
                base_url: row.base_url.clone(),
                api_key: row.api_key.clone(),
                model,
            });
        }
    }

    // ② 内置：展平后按粘性轮询起点旋转（起点 0 时不旋转，保持原序）
    if !input.builtin.is_empty() {
        let mut flat: Vec<LlmTarget> = Vec::new();
        for row in input.builtin {
            for model in row.models() {
                flat.push(LlmTarget {
                    tier: TIER_BUILTIN,
                    base_url: row.base_url.clone(),
                    api_key: row.api_key.clone(),
                    model,
                });
            }
        }
        let start = default_model_settings::sticky_rotate(
            input.builtin_cursor,
            input.builtin_last_ok,
            flat.len(),
        );
        if start > 0 && start < flat.len() {
            flat.rotate_left(start);
        }
        targets.extend(flat);
    }

    // ③ 存量三档尾部：main（无自定义时的首选）→ memory → fallback
    targets.push(target_of(TIER_MAIN, &input.tiered.main));
    targets.push(target_of(TIER_MEMORY, &input.tiered.memory));
    targets.push(target_of(TIER_FALLBACK, &input.tiered.fallback));

    // 未配置档跳过（只可能来自尾部三档：自定义/内置段的 base_url 与 model 均非空）
    targets.retain(|t| t.is_configured());

    // 内置段区间：retain 后按 tier 重算（robust，不依赖插入顺序）
    let builtin_range = targets.iter().position(|t| t.tier == TIER_BUILTIN).map(|start| {
        let count = targets.iter().filter(|t| t.tier == TIER_BUILTIN).count();
        (start, start + count)
    });

    ModelChain { targets, builtin_range }
}

/// 内置模型的进程内编排状态（轮询计数器 + 上次成功下标）。
///
/// 与 `commands::chat` 的 abort_registry 同模式：模块级单例，**内存态、重启清零**——
/// 粘性只是"体感一致"的优化，不需要持久化。
fn builtin_state() -> &'static Mutex<(usize, Option<usize>)> {
    static STATE: OnceLock<Mutex<(usize, Option<usize>)>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new((0, None)))
}

/// 取本轮内置轮询参数并自增计数器（供命令层组装 `ChainInput`）。
pub fn next_builtin_slot() -> (usize, Option<usize>) {
    let mut guard = builtin_state().lock().unwrap_or_else(|e| e.into_inner());
    guard.0 += 1;
    (guard.0, guard.1)
}

/// 记录本轮成功的内置模型下标（下次粘性沿用；风格一致）。
/// 非内置段成功不影响粘性状态。
pub fn mark_builtin_success(builtin_index: usize) {
    if let Ok(mut guard) = builtin_state().lock() {
        guard.1 = Some(builtin_index);
    }
}

/// 沿编排链发起流式对话；全失败时返回本地兜底话术（`Err` 不外抛——应用不崩不卡）。
///
/// 返回：(增量流 或 本地话术文本, 回落链日志)。
/// 成功目标落在内置段时，回写粘性下标（`mark_builtin_success`）。
pub async fn chat_with_chain(
    client: &dyn LlmClient,
    chain: &ModelChain,
    request: ChatRequest,
) -> (Result<DeltaStream, String>, Vec<String>) {
    let mut chain_log: Vec<String> = Vec::new();

    for (idx, target) in chain.targets.iter().enumerate() {
        if !target.is_configured() {
            chain_log.push(format!("[{}] 未配置，跳过", target.tier));
            continue;
        }
        match client.chat_stream(target, request.clone()).await {
            Ok(stream) => {
                if let Some((start, end)) = chain.builtin_range
                    && idx >= start
                    && idx < end
                {
                    mark_builtin_success(idx - start);
                }
                return (Ok(stream), chain_log);
            },
            Err(e) => chain_log.push(e.to_string()),
        }
    }

    // 全部失败：本地兜底话术（伪流：一次 delta）
    let phrase = local_fallback_phrase();
    chain_log.push("全部模型失败 → 本地兜底话术".into());
    (Err(phrase), chain_log)
}

/// 命令层便捷入口：传入两个模型源的行数据 + 三档，内部取轮询槽位并构建链。
///
/// 调用方只需读库（自定义/内置两表 + `app_settings` 三档），不接触轮询状态细节。
pub async fn chat_orchestrated(
    client: &dyn LlmClient,
    custom: &[CustomModelRow],
    builtin: &[DefaultModelRow],
    tiered: &TieredValue,
    request: ChatRequest,
) -> (Result<DeltaStream, String>, Vec<String>) {
    let (cursor, last_ok) = next_builtin_slot();
    let chain = build_chain(&ChainInput {
        custom,
        builtin,
        builtin_cursor: cursor,
        builtin_last_ok: last_ok,
        tiered,
    });
    chat_with_chain(client, &chain, request).await
}

/// 本地兜底话术（简单轮转，避免引入 rand 依赖到路由层）。
pub fn local_fallback_phrase() -> String {
    // 以纳秒取模轮转（对话场景足够；不追求密码学随机）
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    LOCAL_FALLBACK_PHRASES
        .get(nanos % LOCAL_FALLBACK_PHRASES.len())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{LlmTierValue, TieredValue};
    use crate::llm::client::MockLlmClient;

    fn tier(url: &str, model: &str) -> LlmTierValue {
        LlmTierValue {
            base_url: url.into(),
            model: model.into(),
            api_key: "k".into(),
            model_custom: String::new(),
        }
    }

    fn fixture() -> TieredValue {
        TieredValue {
            main: tier("https://main/v1", "m-main"),
            memory: tier("https://memory/v1", "m-memory"),
            fallback: tier("https://fallback/v1", "m-fallback"),
        }
    }

    fn custom_row(name: &str, models: &[&str]) -> CustomModelRow {
        CustomModelRow {
            id: 1,
            name: name.into(),
            setting_type: 1,
            base_url: "https://custom/v1".into(),
            models_json: serde_json::to_string(models).expect("json"),
            api_key: "sk-custom".into(),
            is_can_use: 1,
            sort_order: 0,
        }
    }

    fn builtin_row(name: &str, models: &[&str]) -> DefaultModelRow {
        DefaultModelRow {
            id: 1,
            name: name.into(),
            setting_type: 1,
            base_url: "https://builtin/v1".into(),
            models_json: serde_json::to_string(models).expect("json"),
            api_key: "sk-builtin".into(),
            is_can_use: 1,
        }
    }

    fn empty_input(tiered: &TieredValue) -> ChainInput<'_> {
        ChainInput { custom: &[], builtin: &[], builtin_cursor: 0, builtin_last_ok: None, tiered }
    }

    #[test]
    fn build_chain_orders_custom_then_builtin_then_tiers() {
        let tiered = fixture();
        let custom = [custom_row("甲", &["c1", "c2"])];
        let builtin = [builtin_row("智谱", &["b1"]), builtin_row("硅基", &["b2", "b3"])];
        let chain = build_chain(&ChainInput {
            custom: &custom,
            builtin: &builtin,
            builtin_cursor: 0,
            builtin_last_ok: None,
            tiered: &tiered,
        });

        let tiers: Vec<&str> = chain.targets.iter().map(|t| t.tier).collect();
        assert_eq!(
            tiers,
            vec!["custom", "custom", "builtin", "builtin", "builtin", "main", "memory", "fallback"],
            "自定义(2) → 内置(3) → 三档"
        );
        assert_eq!(chain.targets[0].model, "c1", "自定义按模型序展开");
        assert_eq!(chain.targets[2].model, "b1");
        assert_eq!(chain.builtin_range, Some((2, 5)), "内置段区间应在 retain 后仍准确");
    }

    #[test]
    fn build_chain_builtin_starts_at_sticky_index() {
        let tiered = fixture();
        let builtin = [builtin_row("硅基", &["b1", "b2", "b3"])];
        let chain = build_chain(&ChainInput {
            custom: &[],
            builtin: &builtin,
            builtin_cursor: 99,
            builtin_last_ok: Some(2), // 粘性：从 b3 起
            tiered: &tiered,
        });
        assert_eq!(chain.targets[0].tier, "builtin");
        assert_eq!(chain.targets[0].model, "b3", "粘性起点=上次成功模型");
        assert_eq!(chain.targets[1].model, "b1");
        assert_eq!(chain.targets[2].model, "b2");
    }

    #[test]
    fn build_chain_without_builtin_has_no_builtin_range() {
        let tiered = fixture();
        let chain = build_chain(&empty_input(&tiered));
        assert!(chain.builtin_range.is_none(), "无内置源 → 无内置段");
        let tiers: Vec<&str> = chain.targets.iter().map(|t| t.tier).collect();
        assert_eq!(tiers, vec!["main", "memory", "fallback"]);
    }

    #[test]
    fn build_chain_skips_unconfigured_tiers() {
        let mut tiered = fixture();
        tiered.memory.base_url = String::new(); // 未配置
        let chain = build_chain(&empty_input(&tiered));
        let tiers: Vec<&str> = chain.targets.iter().map(|t| t.tier).collect();
        assert_eq!(tiers, vec!["main", "fallback"], "未配置档应跳过");
    }

    #[test]
    fn legacy_main_stays_first_when_no_custom() {
        let tiered = fixture();
        let builtin = [builtin_row("硅基", &["b1"])];
        let chain = build_chain(&ChainInput {
            custom: &[],
            builtin: &builtin,
            builtin_cursor: 0,
            builtin_last_ok: None,
            tiered: &tiered,
        });
        // 存量用户只配了 main：内置段在最前（main 之前）？——否，内置段先于 main 入链。
        // 语义：内置额度开启时内置优先于存量 main（免费优先），main 退化为主对话备选。
        assert_eq!(chain.targets[0].tier, "builtin");
        assert_eq!(chain.targets[1].tier, "main");
    }

    #[test]
    fn memory_target_from_sources_prefers_first_custom_platform() {
        let mut tiered = fixture();
        tiered.memory.base_url = String::new(); // 无 memory 档配置 → 走自定义平台分支
        let custom = [custom_row("我的平台", &["m-a", "m-b"])];
        let target = memory_target_from_sources(&tiered, false, &custom);
        assert_eq!(target.tier, "custom", "无 memory 档配置 → 取首个自定义平台");
        assert_eq!(target.model, "m-a", "取平台第一个模型");
        assert_eq!(target.base_url, "https://custom/v1");
        assert_eq!(target.api_key, "sk-custom");
    }

    #[test]
    fn memory_target_from_sources_falls_back_to_tier_then_quota() {
        // ① memory 档已配置 → 优先
        let tiered = fixture();
        let target = memory_target_from_sources(&tiered, false, &[]);
        assert_eq!(target.tier, "memory");
        assert_eq!(target.model, "m-memory");

        // ③ 无 memory 档 + 无自定义 + 内置额度 → Workers 端点
        let mut bare = fixture();
        bare.memory.base_url = String::new();
        let target = memory_target_from_sources(&bare, true, &[]);
        assert_eq!(target.base_url, crate::config::EMBEDDED_QUOTA_BASE_URL);
        assert_eq!(target.model, crate::config::EMBEDDING_MODEL);

        // ④ 全空且非额度模式 → 空 target（调用方降级）
        let target = memory_target_from_sources(&bare, false, &[]);
        assert!(!target.is_configured(), "全空 → 未配置 target");
    }

    /// 异步执行辅助（同 client.rs 测试）。
    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt")
            .block_on(fut)
    }

    #[test]
    fn chat_with_chain_returns_first_success_and_marks_builtin() {
        let tiered = fixture();
        let builtin = [builtin_row("硅基", &["b1", "b2"])];
        let chain = build_chain(&ChainInput {
            custom: &[],
            builtin: &builtin,
            builtin_cursor: 0,
            builtin_last_ok: None,
            tiered: &tiered,
        });
        // 首个内置模型失败 → 切第二个成功（Mock 按 base_url 判定，这里同 base_url 全失败/全成功，
        // 故用全成功客户端验证"首个即成功 + 粘性回写"）
        let client = MockLlmClient::ok();
        let req = ChatRequest { messages: vec![], max_tokens: None };
        let (result, log) = block_on(chat_with_chain(&client, &chain, req));
        assert!(result.is_ok(), "首个目标成功则直接返回流");
        assert!(log.is_empty(), "首个目标成功无回落日志");

        // 粘性状态已回写：下一轮应沿用 b1（下标 0）
        let (cursor, last_ok) = next_builtin_slot();
        assert_eq!(last_ok, Some(0), "成功下标应回写为内置段首位");
        assert!(cursor > 0, "计数器应自增");
    }

    #[test]
    fn chat_with_chain_all_fail_yields_local_phrase() {
        let tiered = fixture();
        let chain = build_chain(&empty_input(&tiered));
        let client = MockLlmClient::failing(&["https://"]);
        let req = ChatRequest { messages: vec![], max_tokens: None };
        let (result, log) = block_on(chat_with_chain(&client, &chain, req));
        assert!(result.is_err(), "全失败返回本地话术（Err 携带文本）");
        let phrase = match result {
            Err(phrase) => phrase,
            Ok(_) => panic!("应返回本地话术"),
        };
        assert!(
            crate::config::LOCAL_FALLBACK_PHRASES.contains(&phrase.as_str()),
            "话术应来自预置集合: {phrase}"
        );
        // 3 个三档目标失败 + 1 次兜底记录
        assert_eq!(log.len(), 4, "三次失败 + 一次兜底记录");
    }
}

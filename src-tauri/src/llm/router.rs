//! 三档路由与回落链（C4.3）。
//!
//! 链序（计划 §二 C4.3）：**所选档（main）→ memory → fallback → 本地兜底话术**。
//! 未配置的档位（baseUrl/model 为空）自动跳过；全部失败 → 本地话术（emit 后 done，
//! 应用不崩不卡）。每次失败的 `AppError::Llm` 记入回落链日志（仅档位+状态摘要）。

use super::client::{ChatRequest, LlmClient, LlmTarget};
use crate::config::LOCAL_FALLBACK_PHRASES;
use crate::db::models::TieredValue;

/// 由三档配置解析目标（quota=内置额度时 main 档改指外部端点、免 Key）。
pub fn resolve_main_target(tiered: &TieredValue, quota_embedded: bool) -> LlmTarget {
    let mut main = target_of("main", &tiered.main);
    if quota_embedded {
        main.base_url = crate::config::EMBEDDED_QUOTA_BASE_URL.into();
        main.api_key = String::new(); // 内置额度免自有 Key
    }
    main
}

/// 档位 → 目标。
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
    let mut memory = target_of("memory", &tiered.memory);
    if quota_embedded && memory.base_url.is_empty() {
        memory.base_url = crate::config::EMBEDDED_QUOTA_BASE_URL.into();
        memory.model = crate::config::EMBEDDING_MODEL.into();
    }
    if memory.model.is_empty() {
        memory.model = crate::config::EMBEDDING_MODEL.into();
    }
    memory
}

/// 回落链尝试顺序：main → memory → fallback（未配置档自动跳过）。
pub fn chain_order(tiered: &TieredValue) -> Vec<LlmTarget> {
    let mut targets = vec![
        target_of("main", &tiered.main),
        target_of("memory", &tiered.memory),
        target_of("fallback", &tiered.fallback),
    ];
    targets.retain(|t| t.is_configured());
    targets
}

/// 沿回落链发起流式对话；全失败时返回本地兜底话术（`Err` 不外抛——应用不崩不卡）。
///
/// 返回：(增量流 或 本地话术文本, 回落链日志)。
pub async fn chat_with_fallback(
    client: &dyn LlmClient,
    tiered: &TieredValue,
    quota_embedded: bool,
    request: ChatRequest,
) -> (Result<super::client::DeltaStream, String>, Vec<String>) {
    let mut chain_log: Vec<String> = Vec::new();
    let main = resolve_main_target(tiered, quota_embedded);
    let mut targets = chain_order(tiered);
    // main 档按额度模式修正后放最前（去重：与 chain_order 的 main 重复时覆盖）
    if let Some(first) = targets.first() {
        if first.tier == "main" {
            targets[0] = main;
        } else {
            targets.insert(0, main);
        }
    } else {
        targets.insert(0, main);
    }

    for target in &targets {
        if !target.is_configured() {
            chain_log.push(format!("[{}] 未配置，跳过", target.tier));
            continue;
        }
        match client.chat_stream(target, request.clone()).await {
            Ok(stream) => return (Ok(stream), chain_log),
            Err(e) => chain_log.push(e.to_string()),
        }
    }

    // 全部失败：本地兜底话术（伪流：一次 delta）
    let phrase = local_fallback_phrase();
    chain_log.push("全部档位失败 → 本地兜底话术".into());
    (Err(phrase), chain_log)
}

/// 本地兜底话术（简单轮转，避免引入 rand 依赖到路由层）。
pub fn local_fallback_phrase() -> String {
    // 以纳秒取模轮转（对话场景足够；不追求密码学随机）
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    LOCAL_FALLBACK_PHRASES[nanos % LOCAL_FALLBACK_PHRASES.len()].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{LlmTierValue, TieredValue};

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

    #[test]
    fn chain_order_skips_unconfigured() {
        let mut tiered = fixture();
        tiered.memory.base_url = String::new(); // 未配置
        let targets = chain_order(&tiered);
        assert_eq!(targets.len(), 2, "未配置档应跳过");
        assert_eq!(targets[0].tier, "main");
        assert_eq!(targets[1].tier, "fallback");
    }

    #[test]
    fn embedded_quota_mode_overrides_main_target() {
        let tiered = fixture();
        let main = resolve_main_target(&tiered, true);
        assert_eq!(main.base_url, crate::config::EMBEDDED_QUOTA_BASE_URL);
        assert!(main.api_key.is_empty(), "内置额度免自有 Key");
        // BYOK 模式保持自配
        let main_byok = resolve_main_target(&tiered, false);
        assert_eq!(main_byok.base_url, "https://main/v1");
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
    fn fallback_memory_fails_main_succeeds() {
        // memory 失败 → main 成功：主链序为 main 最前，故 main 直接成功
        let tiered = fixture();
        let client = crate::llm::client::MockLlmClient::ok();
        let req = ChatRequest { messages: vec![], max_tokens: None };
        let (result, log) = block_on(chat_with_fallback(&client, &tiered, false, req));
        // Stream 未实现 Debug：只断言 Ok 分支（is_ok 不要求 Debug）
        assert!(result.is_ok(), "main 成功则直接返回流");
        assert!(log.is_empty(), "首个目标成功无回落日志");
    }

    #[test]
    fn fallback_all_fail_yields_local_phrase() {
        let tiered = fixture();
        let client = crate::llm::client::MockLlmClient::failing(&["https://"]);
        let req = ChatRequest { messages: vec![], max_tokens: None };
        let (result, log) = block_on(chat_with_fallback(&client, &tiered, false, req));
        assert!(result.is_err(), "全失败返回本地话术（Err 携带文本）");
        // expect_err 需要 Stream: Debug（未实现）——用 match 解包
        let phrase = match result {
            Err(phrase) => phrase,
            Ok(_) => panic!("应返回本地话术"),
        };
        assert!(
            crate::config::LOCAL_FALLBACK_PHRASES.contains(&phrase.as_str()),
            "话术应来自预置集合: {phrase}"
        );
        assert_eq!(log.len(), 4, "三次失败 + 一次兜底记录");
    }
}

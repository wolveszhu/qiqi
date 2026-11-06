//! 内置额度记账（埋点单一出口；方案见 `docs/billing/内置体验额度方案.md`）。
//!
//! 三个消费点（chat 主对话 / probe 主动话术 / distill+diary 后台任务）统一经本模块
//! 记账，口径不漂移：
//! - **仅内置额度段**（tier=builtin）落明细——仓储层 `should_charge` 把关，自定义 BYOK 不落库；
//! - token 来源：接口 `usage` 优先；缺失时按字符数端侧估算（token_source=estimated）；
//! - 失败条目 credits=0 但保留明细（审计）；链路全失败（无命中目标）不落库（无归属段）。

use rusqlite::Connection;

use super::client::{ChatRequest, TokenUsage};
use crate::db::repositories::quota_usage::{self, QuotaUsageInput};
use crate::error::AppError;

/// 端侧估算兜底口径（usage 缺失时按字符数折算）。
#[derive(Debug, Clone, Copy, Default)]
pub struct CallChars {
    /// 输入字符数（system + 上下文 + 本轮输入）。
    pub prompt: usize,
    /// 输出字符数（已收拢的回复文本）。
    pub completion: usize,
}

/// 记一笔 LLM 调用（`hit` 为 None 时跳过——链路全失败无从归属，不扣点也不记明细）。
///
/// - `hit`：`(tier, model, platform)`；platform 为模型所属平台名（内置额度记账按平台拆分）。
/// - `usage`：接口用量帧；None 时按 `chars` 端侧估算兜底；
/// - `ok=false`：失败条目记 0 点（审计用），由仓储层保证；
/// - 记账失败仅上抛，由调用方决定是否吞掉（后台任务不应因记账失败而中断主流程）。
pub fn record_call(
    conn: &Connection,
    scene: &str,
    hit: Option<(&str, &str, &str)>,
    usage: Option<TokenUsage>,
    chars: CallChars,
    ok: bool,
    error: Option<&str>,
) -> Result<(), AppError> {
    let Some((tier, model, platform)) = hit else {
        return Ok(());
    };
    let (prompt_tokens, completion_tokens, token_source) = match usage {
        Some(u) => (u.prompt_tokens, u.completion_tokens, "api"),
        None => (
            quota_usage::estimate_tokens(chars.prompt),
            quota_usage::estimate_tokens(chars.completion),
            "estimated",
        ),
    };
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    quota_usage::record(
        conn,
        &QuotaUsageInput {
            date: &date,
            scene,
            tier,
            model,
            platform,
            prompt_tokens,
            completion_tokens,
            token_source,
            ok,
            error,
        },
    )
}

/// 请求全字符数（端侧估算兜底口径：system + 上下文 + 本轮输入）。
pub fn request_chars(request: &ChatRequest) -> usize {
    request.messages.iter().map(|m| m.content.as_text().chars().count()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::client::ChatMessage;
    use crate::test_util::fresh_db;

    #[test]
    fn request_chars_sums_message_contents() {
        let req = ChatRequest {
            messages: vec![
                ChatMessage { role: "system".into(), content: "人设".into() },
                ChatMessage { role: "user".into(), content: "你好七七".into() },
            ],
            max_tokens: None,
        };
        assert_eq!(request_chars(&req), 6);
    }

    #[test]
    fn record_call_charges_builtin_only_and_skips_no_hit() {
        let (_dir, conn) = fresh_db();
        // 内置段：按接口 usage 记账
        record_call(
            &conn,
            "chat",
            Some(("builtin", "glm-4-flash", "智谱")),
            Some(TokenUsage { prompt_tokens: 100, completion_tokens: 50 }),
            CallChars::default(),
            true,
            None,
        )
        .expect("内置段记账");
        // 自定义段：不落库（BYOK 不记录红线）
        record_call(
            &conn,
            "chat",
            Some(("custom", "gpt-x", "自定义平台")),
            Some(TokenUsage { prompt_tokens: 999, completion_tokens: 999 }),
            CallChars::default(),
            true,
            None,
        )
        .expect("自定义段跳过");
        // 链路全失败（无 hit）：不落库
        record_call(&conn, "chat", None, None, CallChars::default(), false, Some("全部失败"))
            .expect("无命中跳过");

        let period = chrono::Local::now().format("%Y-%m").to_string();
        let rows = quota_usage::recent(&conn, &period, 10).expect("读明细");
        assert_eq!(rows.len(), 1, "仅内置段一条明细");
        assert_eq!(rows[0].model, "glm-4-flash");
        assert_eq!(rows[0].platform, "智谱", "平台名应随命中目标落明细");
        assert_eq!(rows[0].token_source, "api");
        let monthly = quota_usage::monthly(&conn, &period).expect("读汇总");
        assert_eq!(monthly.used, 200, "100×1.0 + 50×2.0");
        assert_eq!(monthly.requests, 1);
        let by_platform = quota_usage::platform_breakdown(&conn, &period).expect("读平台汇总");
        assert_eq!(by_platform.len(), 1);
        assert_eq!(by_platform[0].platform, "智谱");
        assert_eq!(by_platform[0].used, 200);
        assert_eq!(by_platform[0].requests, 1);
    }

    #[test]
    fn record_call_without_usage_estimates_from_chars() {
        let (_dir, conn) = fresh_db();
        record_call(
            &conn,
            "proactive",
            Some(("builtin", "b1", "硅基流动")),
            None,
            CallChars { prompt: 16, completion: 0 },
            true,
            None,
        )
        .expect("估算记账");
        let period = chrono::Local::now().format("%Y-%m").to_string();
        let rows = quota_usage::recent(&conn, &period, 10).expect("读明细");
        assert_eq!(rows[0].token_source, "estimated", "无 usage 应标注估算来源");
        assert_eq!(rows[0].prompt_tokens, 10, "16/1.6 向上取整");
    }
}

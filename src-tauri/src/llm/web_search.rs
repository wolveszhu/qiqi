//! 联网搜索（应用侧检索增强；C4 扩展）。
//!
//! 路线（计划确认）：Rust 侧调用搜索 API 取结果，注入 system prompt 上下文——
//! **非模型原生 tool call**，故所有 OpenAI 兼容模型通用（含内置免费 GLM-4-flash），
//! 且不改动既有流式对话循环。
//!
//! 服务商（首期）：Tavily / Brave / SerpAPI，由 `search_settings.provider` 选择；
//! 解析函数纯函数化、便于单测用 fixture JSON 验证。超时 10s 连接 / 30s 总
//! （复用 `config::SEARCH_TIMEOUT_SECS`，与 client 连接池口径一致）。
//!
//! 红线（rule.md §H / settings 同口径）：`api_key` 仅在此层与仓储之间流转，
//! **禁止进日志/错误详情**（错误只带 HTTP 状态与响应前缀截断）。

use std::time::Duration;

use serde_json::Value;

use crate::config::{SEARCH_MAX_RESULTS, SEARCH_SNIPPET_MAX_CHARS, SEARCH_TIMEOUT_SECS};
use crate::db::repositories::search_settings::{SearchProvider, SearchSettingsRow};
use crate::error::AppError;

/// 单条搜索结果（title/url 用于引用展示，snippet 用于注入 system 上下文）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// 发起联网搜索：返回结构化结果列表（已截断 snippet、已限条数）。
///
/// 前置由调用方保证 `settings.is_ready()`；此处再保险校验（未启用/无 Key 直接 Err，
/// 调用方据此静默回落普通对话，不中断用户）。
pub async fn web_search(
    settings: &SearchSettingsRow,
    query: &str,
) -> Result<Vec<SearchResult>, AppError> {
    if !settings.is_ready() {
        return Err(AppError::Config("联网搜索未启用或未配置 Key".into()));
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(SEARCH_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Llm(format!("搜索客户端初始化失败: {e}")))?;

    let max_results = (settings.max_results as usize).clamp(1, SEARCH_MAX_RESULTS);
    match settings.provider_enum() {
        SearchProvider::Tavily => {
            tavily_request(&client, &settings.api_key, query, max_results).await
        },
        SearchProvider::Brave => {
            brave_request(&client, &settings.api_key, query, max_results).await
        },
        SearchProvider::Serpapi => {
            serpapi_request(&client, &settings.api_key, query, max_results).await
        },
    }
}

/// 统一状态校验（内联于各请求函数，错误分支才消费 body 并 return，成功分支保留 `resp` 供 `.json()`）。
/// 非成功 → `AppError::Llm`（只带状态与响应前缀，禁 Key）。
macro_rules! ensure_ok {
    ($resp:ident) => {{
        if !$resp.status().is_success() {
            let status = $resp.status();
            let body = $resp.text().await.unwrap_or_default();
            let prefix: String = body.chars().take(160).collect();
            return Err(AppError::Llm(format!("搜索 API [{status}]: {prefix}")));
        }
    }};
}

// ---------- Tavily（POST JSON） ----------

async fn tavily_request(
    client: &reqwest::Client,
    key: &str,
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>, AppError> {
    let resp = client
        .post("https://api.tavily.com/search")
        .json(&serde_json::json!({
            "api_key": key,
            "query": query,
            "max_results": max_results,
            "search_depth": "basic",
            "include_raw_content": false,
        }))
        .send()
        .await
        .map_err(|e| AppError::Llm(format!("Tavily 请求失败: {e}")))?;
    ensure_ok!(resp);
    let v: Value = resp
        .json()
        .await
        .map_err(|e| AppError::Llm(format!("Tavily 响应解析失败: {e}")))?;
    Ok(parse_tavily(&v))
}

/// Tavily 响应：`{results:[{title,url,content}]}`。
pub(crate) fn parse_tavily(v: &Value) -> Vec<SearchResult> {
    v.get("results")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let title = item.get("title")?.as_str()?.to_string();
                    let url = item.get("url")?.as_str()?.to_string();
                    let snippet = item
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(SearchResult { title, url, snippet: truncate(&snippet) })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------- Brave（GET + 头 Token） ----------

async fn brave_request(
    client: &reqwest::Client,
    key: &str,
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>, AppError> {
    let resp = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .query(&[("q", query), ("count", &max_results.to_string())])
        .header("X-Subscription-Token", key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Llm(format!("Brave 请求失败: {e}")))?;
    ensure_ok!(resp);
    let v: Value = resp
        .json()
        .await
        .map_err(|e| AppError::Llm(format!("Brave 响应解析失败: {e}")))?;
    Ok(parse_brave(&v))
}

/// Brave 响应：`{web:{results:[{title,url,description}]}}`。
pub(crate) fn parse_brave(v: &Value) -> Vec<SearchResult> {
    v.get("web")
        .and_then(|w| w.get("results"))
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let title = item.get("title")?.as_str()?.to_string();
                    let url = item.get("url")?.as_str()?.to_string();
                    let snippet = item
                        .get("description")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(SearchResult { title, url, snippet: truncate(&snippet) })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------- SerpAPI（GET query 带 key） ----------

async fn serpapi_request(
    client: &reqwest::Client,
    key: &str,
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>, AppError> {
    let resp = client
        .get("https://serpapi.com/search")
        .query(&[
            ("engine", "google"),
            ("q", query),
            ("num", &max_results.to_string()),
            ("api_key", key),
        ])
        .send()
        .await
        .map_err(|e| AppError::Llm(format!("SerpAPI 请求失败: {e}")))?;
    ensure_ok!(resp);
    let v: Value = resp
        .json()
        .await
        .map_err(|e| AppError::Llm(format!("SerpAPI 响应解析失败: {e}")))?;
    Ok(parse_serpapi(&v))
}

/// SerpAPI 响应：`{organic_results:[{title,link,snippet}]}`。
pub(crate) fn parse_serpapi(v: &Value) -> Vec<SearchResult> {
    v.get("organic_results")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let title = item.get("title")?.as_str()?.to_string();
                    let url = item.get("link")?.as_str()?.to_string();
                    let snippet = item
                        .get("snippet")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(SearchResult { title, url, snippet: truncate(&snippet) })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 单条 snippet 截断（防单条过长撑爆 prompt；约 400 字 ≈ 250 token）。
fn truncate(s: &str) -> String {
    if s.chars().count() <= SEARCH_SNIPPET_MAX_CHARS {
        s.to_string()
    } else {
        s.chars().take(SEARCH_SNIPPET_MAX_CHARS).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tavily_extracts_results_and_truncates() {
        let json: Value = serde_json::from_str(
            r#"{"results":[
                {"title":"A","url":"https://a.test","content":"短摘要"},
                {"title":"B","url":"https://b.test","content":"0123456789"},
                {"title":"C","url":"https://c.test"}
            ]}"#,
        )
        .expect("json");
        let r = parse_tavily(&json);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].title, "A");
        assert_eq!(r[0].url, "https://a.test");
        assert_eq!(r[0].snippet, "短摘要");
        // 缺 content → 空 snippet（不丢条目）
        assert_eq!(r[2].snippet, "");
    }

    #[test]
    fn parse_brave_reads_web_results() {
        let json: Value = serde_json::from_str(
            r#"{"web":{"results":[
                {"title":"X","url":"https://x.test","description":"desc-x"}
            ]}}"#,
        )
        .expect("json");
        let r = parse_brave(&json);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].snippet, "desc-x");
    }

    #[test]
    fn parse_serpapi_reads_organic_results() {
        let json: Value = serde_json::from_str(
            r#"{"organic_results":[
                {"title":"S","link":"https://s.test","snippet":"snip-s"}
            ]}"#,
        )
        .expect("json");
        let r = parse_serpapi(&json);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].url, "https://s.test");
        assert_eq!(r[0].snippet, "snip-s");
    }

    #[test]
    fn parse_handles_missing_array_gracefully() {
        let json = serde_json::json!({"foo": "bar"});
        assert!(parse_tavily(&json).is_empty());
        assert!(parse_brave(&json).is_empty());
        assert!(parse_serpapi(&json).is_empty());
    }

    #[test]
    fn truncate_respects_limit() {
        let long = "字".repeat(SEARCH_SNIPPET_MAX_CHARS + 50);
        let out = truncate(&long);
        assert_eq!(out.chars().count(), SEARCH_SNIPPET_MAX_CHARS);
        let short = "短文本";
        assert_eq!(truncate(short), short);
    }

    #[test]
    fn settings_is_ready_gate() {
        let off = SearchSettingsRow {
            provider: "tavily".into(),
            api_key: "k".into(),
            enabled: false,
            max_results: 5,
            lang: "zh".into(),
        };
        assert!(!off.is_ready(), "未启用不应就绪");
        let no_key = SearchSettingsRow {
            provider: "tavily".into(),
            api_key: String::new(),
            enabled: true,
            max_results: 5,
            lang: "zh".into(),
        };
        assert!(!no_key.is_ready(), "无 Key 不应就绪");
        let ready = SearchSettingsRow {
            provider: "tavily".into(),
            api_key: "k".into(),
            enabled: true,
            max_results: 5,
            lang: "zh".into(),
        };
        assert!(ready.is_ready());
    }
}

//! 划词翻译编排：配置 → 语种判定 → provider 调用 → 内置额度记账 → 结果 / 重试。
//!
//! 纪律：DB 锁只在同步块内持有，**绝不跨 await**；凭据只在内存短暂停留，
//! 不进事件、日志、DTO 与错误信息。自定义通道传 `tier=custom`，由 `should_charge` 跳过记账。

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use rusqlite::Connection;

use crate::db::models::{SaveTranslateSettingsPayload, TranslateResultDto, TranslateSettingsDto};
use crate::db::repositories::{quota_usage, translate_settings as repo};
use crate::error::AppError;
use crate::llm::quota::{CallChars, record_call};
use crate::state::AppState;

use super::provider::{Lang, TranslateRequest, TranslationOutcome, http_client, provider_for};

/// 重新翻译缓存条目（仅存原文与译入语言，不存凭据）。
#[derive(Debug, Clone)]
struct RetryEntry {
    text: String,
    target_lang: String,
}

/// 进程内重试表：与 `commands/chat.rs` 的会话表同风格，不扩 AppState。
fn retry_cache() -> &'static Mutex<HashMap<String, RetryEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, RetryEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 一次翻译的执行计划（同步阶段组装，脱离 DB 锁后再发 HTTP）。
#[derive(Debug, Clone)]
struct Plan {
    text: String,
    source: Lang,
    target: Lang,
    provider_id: String,
    is_builtin: bool,
    app_id: String,
    secret: String,
}

fn lock_db(state: &AppState) -> Result<MutexGuard<'_, Connection>, AppError> {
    state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))
}

/// 源语言判定：本版仅中英互译，含汉字判中文、纯拉丁字母判英文，其它语种直接拒绝。
pub fn detect_source(text: &str) -> Result<Lang, AppError> {
    let mut has_han = false;
    let mut has_latin = false;
    for c in text.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&c) {
            has_han = true;
        } else if c.is_ascii_alphabetic() {
            has_latin = true;
        } else if c.is_alphabetic() {
            return Err(AppError::Config("本版仅支持中英互译".into()));
        }
    }
    if has_han {
        Ok(Lang::Zh)
    } else if has_latin {
        Ok(Lang::En)
    } else {
        Err(AppError::Config("未识别到可翻译文本".into()))
    }
}

/// 译入语言：`auto` = 与源语言相反；其余只接受 `zh` / `en`。
fn resolve_target(source: Lang, raw: &str) -> Result<Lang, AppError> {
    match raw {
        "auto" => Ok(source.opposite()),
        "zh" => Ok(Lang::Zh),
        "en" => Ok(Lang::En),
        other => Err(AppError::Config(format!("不支持的译入语言: {other}"))),
    }
}

/// 原文校验：去首尾空白、非空、不超过单次请求上限（百度官方建议 2000 字符）。
fn validate_text(text: &str) -> Result<String, AppError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::Config("选中文本为空".into()));
    }
    if trimmed.chars().count() > crate::config::TRANSLATE_MAX_CHARS {
        return Err(AppError::Config("选中文本过长".into()));
    }
    Ok(trimmed.to_string())
}

/// 当月内置额度是否已用尽（仅内置通道调用）。
fn quota_exhausted(conn: &Connection) -> Result<bool, AppError> {
    let period = chrono::Local::now().format("%Y-%m").to_string();
    let monthly = quota_usage::monthly(conn, &period)?;
    Ok(monthly.used >= monthly.granted)
}

fn new_result_id() -> String {
    let mut buf = [0u8; 12];
    let _ = getrandom::getrandom(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// 同步阶段：读配置、校验、选凭据、判语种与额度，组装执行计划。
fn prepare(conn: &Connection, text: &str, override_lang: Option<&str>) -> Result<Plan, AppError> {
    let text = validate_text(text)?;
    let row = repo::get(conn)?;
    if !row.enabled {
        return Err(AppError::Config("划词翻译已关闭".into()));
    }
    let provider_id = row.provider;
    let is_builtin = provider_id == "baidu";
    let (app_id, secret) = if is_builtin {
        (row.builtin_app_id, row.builtin_secret)
    } else {
        (row.custom_app_id, row.custom_api_key)
    };
    let credential_missing = secret.is_empty()
        || ((provider_id == "baidu" || provider_id == "youdao") && app_id.is_empty());
    if credential_missing {
        let message = if is_builtin {
            "内置翻译未配置"
        } else {
            "自定义翻译服务未配置密钥"
        };
        return Err(AppError::Config(message.into()));
    }
    if is_builtin && quota_exhausted(conn)? {
        return Err(AppError::Config("内置翻译额度已用尽".into()));
    }

    let source = detect_source(&text)?;
    let raw_target = override_lang.unwrap_or(row.target_lang.as_str());
    let target = resolve_target(source, raw_target)?;
    Ok(Plan { text, source, target, provider_id, is_builtin, app_id, secret })
}

/// 异步阶段：调用 preset（不持有任何 DB 锁）。
async fn call_provider(plan: &Plan) -> Result<TranslationOutcome, AppError> {
    let provider = provider_for(&plan.provider_id)?;
    let req = TranslateRequest {
        text: &plan.text,
        source: plan.source,
        target: plan.target,
        app_id: &plan.app_id,
        secret: &plan.secret,
    };
    provider.translate(http_client(), req).await
}

/// 收尾阶段：记账（仅内置）、缓存重试条目、构造 DTO。
fn finalize(
    conn: &Connection,
    plan: &Plan,
    outcome: Result<TranslationOutcome, AppError>,
) -> Result<TranslateResultDto, AppError> {
    let (ok, error_text, completion_chars) = match &outcome {
        Ok(value) => (true, None, value.translated_text.chars().count()),
        Err(e) => (false, Some(e.to_string()), 0),
    };
    if plan.is_builtin {
        record_call(
            conn,
            "translate",
            Some(("builtin", plan.provider_id.as_str(), "百度翻译")),
            None,
            CallChars { prompt: plan.text.chars().count(), completion: completion_chars },
            ok,
            error_text.as_deref(),
        )?;
    }
    let outcome = outcome?;
    let result_id = new_result_id();
    {
        let mut cache = retry_cache().lock().map_err(|_| AppError::Db("重试缓存锁异常".into()))?;
        if cache.len() >= 64 {
            cache.clear();
        }
        cache.insert(
            result_id.clone(),
            RetryEntry { text: plan.text.clone(), target_lang: plan.target.as_str().to_string() },
        );
    }
    Ok(TranslateResultDto {
        result_id,
        source_text: plan.text.clone(),
        translated_text: outcome.translated_text,
        source_lang: plan.source.as_str().to_string(),
        target_lang: plan.target.as_str().to_string(),
        provider: plan.provider_id.clone(),
        is_builtin: plan.is_builtin,
        quota_exhausted: if plan.is_builtin { quota_exhausted(conn)? } else { false },
    })
}

/// 执行一次翻译（命令层入口；DB 锁不跨 await）。
pub async fn execute(
    state: &AppState,
    text: &str,
    override_lang: Option<&str>,
) -> Result<TranslateResultDto, AppError> {
    let plan = {
        let conn = lock_db(state)?;
        prepare(&conn, text, override_lang)?
    };
    let outcome = call_provider(&plan).await;
    let conn = lock_db(state)?;
    finalize(&conn, &plan, outcome)
}

/// 重新翻译：用缓存的原文与译入语言重发（每次真实请求各记一条账）。
pub async fn retry(state: &AppState, result_id: &str) -> Result<TranslateResultDto, AppError> {
    let entry = {
        let cache = retry_cache().lock().map_err(|_| AppError::Db("重试缓存锁异常".into()))?;
        cache
            .get(result_id)
            .cloned()
            .ok_or_else(|| AppError::Config("译文已过期，请重新划词".into()))?
    };
    execute(state, &entry.text, Some(&entry.target_lang)).await
}

/// 读取配置（Key 脱敏：只回 `hasCustomKey` + 末 4 位指纹）。
pub fn get_settings(conn: &Connection) -> Result<TranslateSettingsDto, AppError> {
    let row = repo::get(conn)?;
    // 先取借用型字段，避免 String 字段移动后再借用 row（与 search_settings 同口径）。
    let has_builtin = row.has_builtin_credential();
    let has_custom = row.has_custom_key();
    let fingerprint = key_fingerprint(&row.custom_api_key);
    Ok(TranslateSettingsDto {
        enabled: row.enabled,
        target_lang: row.target_lang,
        provider: row.provider,
        has_builtin_credential: has_builtin,
        has_custom_key: has_custom,
        custom_key_fingerprint: fingerprint,
    })
}

/// 保存配置（薄壳转发仓储；Key 缺省不改、空串清空）。
pub fn save_settings(
    conn: &Connection,
    payload: SaveTranslateSettingsPayload,
) -> Result<(), AppError> {
    repo::save(
        conn,
        payload.enabled,
        &payload.target_lang,
        &payload.provider,
        payload.custom_api_key.as_deref(),
        payload.custom_app_id.as_deref(),
    )
}

fn key_fingerprint(key: &str) -> Option<String> {
    if key.len() >= 4 { Some(key[key.len() - 4..].to_string()) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn detect_source_only_accepts_zh_and_en() {
        assert_eq!(detect_source("你好，世界").expect("中文"), Lang::Zh);
        assert_eq!(detect_source("Hello world").expect("英文"), Lang::En);
        assert_eq!(detect_source("你好 world").expect("中英混排"), Lang::Zh);
        assert!(detect_source("こんにちは").is_err(), "其它语种应拒绝");
        assert!(detect_source("12345!!!").is_err(), "无字母文本应拒绝");
    }

    #[test]
    fn resolve_target_auto_is_opposite() {
        assert_eq!(resolve_target(Lang::Zh, "auto").expect("自动"), Lang::En);
        assert_eq!(resolve_target(Lang::En, "auto").expect("自动"), Lang::Zh);
        assert_eq!(resolve_target(Lang::En, "zh").expect("锁定"), Lang::Zh);
        assert!(resolve_target(Lang::En, "ja").is_err());
    }

    #[test]
    fn validate_text_trims_and_rejects_invalid() {
        assert_eq!(validate_text("  hi  ").expect("去空白"), "hi");
        assert!(validate_text("   ").is_err());
        assert!(validate_text(&"a".repeat(crate::config::TRANSLATE_MAX_CHARS + 1)).is_err());
    }

    #[test]
    fn get_settings_never_leaks_keys() {
        let (_dir, conn) = fresh_db();
        repo::save(&conn, true, "auto", "deepl", Some("sk-secret-4321"), Some("app-1"))
            .expect("保存");
        let dto = get_settings(&conn).expect("读取");
        assert!(dto.has_custom_key);
        assert_eq!(dto.custom_key_fingerprint.as_deref(), Some("4321"));
        let json = serde_json::to_string(&dto).expect("序列化");
        assert!(!json.contains("sk-secret"), "DTO 不得含明文 Key");
    }

    #[test]
    fn prepare_rejects_disabled_and_missing_credentials() {
        let (_dir, conn) = fresh_db();
        assert!(prepare(&conn, "Hello", None).is_err(), "未注入内置凭据时应拒绝");
        repo::save(&conn, false, "auto", "baidu", None, None).expect("关闭开关");
        assert!(prepare(&conn, "Hello", None).is_err(), "关闭时不应发起");
    }
}

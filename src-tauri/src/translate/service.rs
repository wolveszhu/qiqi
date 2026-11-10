//! 划词翻译编排：配置 → 语种判定 → 内置候选选路 → provider 调用 / 失败切换 → 记账 → 结果 / 重试。
//!
//! 内置两家（百度 / 讯飞）由 M13 表驱动：按**当月各家用量升序**排候选，达到各自月度上限的
//! 剔除；某家请求失败时依次切换下一家重试一次。每次真实请求各记一条账，`platform` 用服务名，
//! 因此 `quota_usage_platform_monthly` 天然按服务商分列。
//!
//! 纪律：DB 锁只在同步块内持有，**绝不跨 await**；凭据只在内存短暂停留，
//! 不进事件、日志、DTO 与错误信息。自定义通道只有一个候选且不记账。

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use rusqlite::Connection;

use crate::db::models::{SaveTranslateSettingsPayload, TranslateResultDto, TranslateSettingsDto};
use crate::db::repositories::{
    builtin_translate_providers as builtin_repo, custom_translate_providers as custom_repo,
    quota_usage, translate_settings as repo,
};
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

/// 单个候选服务商（凭据只在内存短暂停留）。
#[derive(Debug, Clone)]
struct Candidate {
    provider_id: String,
    /// 实际使用的服务名，同时是额度记账的 `platform` 键。
    service_name: String,
    app_id: String,
    /// 签名密钥（百度密钥 / 讯飞 APISecret / 其它服务的 API Key）。
    secret: String,
    /// 仅讯飞需要的 APIKey。
    api_key: String,
}

/// 一次翻译的执行计划（同步阶段组装，脱离 DB 锁后再发 HTTP）。
#[derive(Debug, Clone)]
struct Plan {
    text: String,
    source: Lang,
    target: Lang,
    is_builtin: bool,
    /// 已按「当月用量升序」排好的候选；内置服务失败时依次切换。
    candidates: Vec<Candidate>,
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

fn current_period() -> String {
    chrono::Local::now().format("%Y-%m").to_string()
}

/// 内置候选：启用 + 凭据齐全 + 本月未达各自上限，按「已用量升序、同量按 sort_order」排序。
///
/// 用量直接取自 `quota_usage_platform_monthly`：记账时 `platform` 用服务名，
/// 只有该服务商的翻译请求会累加这一行。
fn builtin_candidates(conn: &Connection) -> Result<Vec<Candidate>, AppError> {
    let period = current_period();
    let mut ranked: Vec<(i64, i64, Candidate)> = Vec::new();
    for row in builtin_repo::list_all(conn)? {
        if !row.is_usable() {
            continue;
        }
        let (used, _) = quota_usage::platform_used(conn, &period, &row.name)?;
        if used >= row.monthly_granted {
            continue;
        }
        ranked.push((
            used,
            row.sort_order,
            Candidate {
                provider_id: row.provider_id,
                service_name: row.name,
                app_id: row.app_id,
                secret: row.api_secret,
                api_key: row.api_key,
            },
        ));
    }
    ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    Ok(ranked.into_iter().map(|(_, _, candidate)| candidate).collect())
}

/// 是否已有可用内置服务商达到兜底上限（用于结果里的 `quotaExhausted` 提示）。
///
/// 任一家达到每月 5,000 即返回 true：该家已退出候选、由另一家自动接手，
/// 此时在译文弹窗提示用户添加自定义翻译服务商，避免额度耗尽后才被动报错。
fn builtin_any_exhausted(conn: &Connection) -> Result<bool, AppError> {
    let period = current_period();
    for row in builtin_repo::list_all(conn)?.into_iter().filter(|r| r.is_usable()) {
        let (used, _) = quota_usage::platform_used(conn, &period, &row.name)?;
        if used >= row.monthly_granted {
            return Ok(true);
        }
    }
    Ok(false)
}

fn new_result_id() -> String {
    let mut buf = [0u8; 12];
    let _ = getrandom::getrandom(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// 同步阶段：读配置、自填优先、内置按用量选路、判语种与额度，组装候选列表。
fn prepare(conn: &Connection, text: &str, override_lang: Option<&str>) -> Result<Plan, AppError> {
    let text = validate_text(text)?;
    let row = repo::get(conn)?;
    if !row.enabled {
        return Err(AppError::Config("划词翻译已关闭".into()));
    }

    // 自填服务商优先：按 sort_order 取第一个启用且凭据齐全的（BYOK 不计入内置额度）。
    let (is_builtin, candidates) =
        if let Some(custom) = custom_repo::list_usable(conn)?.into_iter().next() {
            (
                false,
                vec![Candidate {
                    provider_id: custom.provider,
                    service_name: custom.name,
                    app_id: custom.app_id,
                    secret: custom.api_key,
                    api_key: String::new(),
                }],
            )
        } else if !row.custom_api_key.is_empty() {
            // 兼容旧版单条自定义配置：用户填过 Key 时仍优先于内置服务。
            let provider_id = row.provider.clone();
            (
                false,
                vec![Candidate {
                    provider_id: provider_id.clone(),
                    service_name: provider_id,
                    app_id: row.custom_app_id,
                    secret: row.custom_api_key,
                    api_key: String::new(),
                }],
            )
        } else {
            // 内置两家：只按各自独立月度上限与启停判定。
            // 不能用 quota_usage_monthly 的全局额度做总闸——它是聊天等所有内置场景共用的，
            // 被别的场景消耗完会误伤翻译（各家明明还有额度却报「额度用尽」）。
            let candidates = builtin_candidates(conn)?;
            if candidates.is_empty() {
                return Err(AppError::Config(
                    "内置翻译服务均不可用：已全部停用 / 凭据缺失 / 本月额度用尽".into(),
                ));
            }
            (true, candidates)
        };

    {
        let first =
            candidates.first().ok_or_else(|| AppError::Config("没有可用的翻译服务".into()))?;
        let requires_app_id = matches!(first.provider_id.as_str(), "baidu" | "youdao" | "xfyun");
        let credential_missing = first.secret.is_empty()
            || (requires_app_id && first.app_id.is_empty())
            || (first.provider_id == "xfyun" && first.api_key.is_empty());
        if credential_missing {
            return Err(AppError::Config("没有可用的翻译服务凭据，请在设置中添加服务商".into()));
        }
    }

    let source = detect_source(&text)?;
    let raw_target = override_lang.unwrap_or(row.target_lang.as_str());
    let target = resolve_target(source, raw_target)?;
    Ok(Plan { text, source, target, is_builtin, candidates })
}

/// 异步阶段：对某个候选调用 preset（不持有任何 DB 锁）。
async fn call_provider(plan: &Plan, candidate: &Candidate) -> Result<TranslationOutcome, AppError> {
    let provider = provider_for(&candidate.provider_id)?;
    let req = TranslateRequest {
        text: &plan.text,
        source: plan.source,
        target: plan.target,
        app_id: &candidate.app_id,
        secret: &candidate.secret,
        api_key: &candidate.api_key,
    };
    provider.translate(http_client(), req).await
}

/// 每次真实请求各记一条账（仅内置）；`platform` 用服务名，两家因此分别统计。
fn record_attempt(
    conn: &Connection,
    plan: &Plan,
    candidate: &Candidate,
    outcome: &Result<TranslationOutcome, AppError>,
) -> Result<(), AppError> {
    if !plan.is_builtin {
        return Ok(());
    }
    let (ok, error_text, completion_chars) = match outcome {
        Ok(value) => (true, None, value.translated_text.chars().count()),
        Err(e) => (false, Some(e.to_string()), 0),
    };
    record_call(
        conn,
        "translate",
        Some(("builtin", candidate.provider_id.as_str(), candidate.service_name.as_str())),
        None,
        CallChars { prompt: plan.text.chars().count(), completion: completion_chars },
        ok,
        error_text.as_deref(),
    )
}

/// 收尾阶段：缓存重试条目、构造 DTO（成功路径才调用）。
fn finalize(
    conn: &Connection,
    plan: &Plan,
    candidate: &Candidate,
    outcome: TranslationOutcome,
) -> Result<TranslateResultDto, AppError> {
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
        provider: candidate.provider_id.clone(),
        service_name: candidate.service_name.clone(),
        is_builtin: plan.is_builtin,
        quota_exhausted: if plan.is_builtin { builtin_any_exhausted(conn)? } else { false },
    })
}

/// 执行一次翻译：候选按用量升序，某家失败自动切换下一家（DB 锁不跨 await）。
pub async fn execute(
    state: &AppState,
    text: &str,
    override_lang: Option<&str>,
) -> Result<TranslateResultDto, AppError> {
    let plan = {
        let conn = lock_db(state)?;
        prepare(&conn, text, override_lang)?
    };
    let mut last_err: Option<AppError> = None;
    for candidate in &plan.candidates {
        let outcome = call_provider(&plan, candidate).await;
        // 记账失败属 DB 问题，直接上抛，不当作 provider 失败去重发请求。
        let conn = lock_db(state)?;
        record_attempt(&conn, &plan, candidate, &outcome)?;
        match outcome {
            Ok(value) => return finalize(&conn, &plan, candidate, value),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| AppError::Config("没有可用的翻译服务".into())))
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
    // 内置凭据真源是 M13 的 builtin_translate_providers，不用 translate_settings
    // 的 M12 历史列：后者被清空 / 轮换时设置页会误报「内置服务未配置」。
    let has_builtin = builtin_repo::list_all(conn)?.iter().any(|r| r.is_usable());
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
    fn get_settings_reports_builtin_availability_from_m13() {
        let (_dir, conn) = fresh_db();
        assert!(get_settings(&conn).expect("读取").has_builtin_credential);
        conn.execute("UPDATE builtin_translate_providers SET is_can_use = 2", [])
            .expect("全部停用");
        // M12 历史列仍有凭据，但对外视图应反映 M13 的真实可用状态。
        assert!(!get_settings(&conn).expect("读取").has_builtin_credential);
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
        // 停用两家内置服务商，模拟「所有凭据都不可用」。
        conn.execute("UPDATE builtin_translate_providers SET is_can_use = 2", [])
            .expect("停用内置服务商");
        assert!(prepare(&conn, "Hello", None).is_err(), "凭据全不可用时应拒绝");
        repo::save(&conn, false, "auto", "baidu", None, None).expect("关闭开关");
        assert!(prepare(&conn, "Hello", None).is_err(), "关闭时不应发起");
    }

    #[test]
    fn prepare_uses_seeded_builtin_candidates_sorted_by_usage() {
        let (_dir, conn) = fresh_db();
        let plan = prepare(&conn, "Hello", None).expect("内置凭据应可用");
        assert!(plan.is_builtin, "内置凭据计入内置额度");
        assert_eq!(plan.candidates.len(), 2, "百度 + 讯飞两家候选");
        assert_eq!(plan.candidates[0].provider_id, "baidu", "同用量按 sort_order");
        assert_eq!(plan.candidates[0].service_name, "百度翻译");
        assert_eq!(plan.candidates[1].provider_id, "xfyun");
        assert_eq!(plan.candidates[1].service_name, "讯飞翻译");
        assert!(!plan.candidates[1].api_key.is_empty(), "讯飞需带 APIKey");

        // 给百度记一笔账后，讯飞用量更低，应排到最前。
        crate::llm::quota::record_call(
            &conn,
            "translate",
            Some(("builtin", "baidu", "百度翻译")),
            None,
            CallChars { prompt: 100, completion: 100 },
            true,
            None,
        )
        .expect("记账");
        assert!(
            quota_usage::platform_used(&conn, &current_period(), "百度翻译").expect("读用量").0 > 0
        );
        let plan = prepare(&conn, "Hello", None).expect("仍应可用");
        assert_eq!(plan.candidates[0].provider_id, "xfyun", "按当月用量最少者优先");
    }

    #[test]
    fn prepare_skips_provider_at_its_own_monthly_cap() {
        let (_dir, conn) = fresh_db();
        conn.execute(
            "UPDATE builtin_translate_providers SET monthly_granted = 1 WHERE provider_id = 'baidu'",
            [],
        )
        .expect("调低百度上限");
        crate::llm::quota::record_call(
            &conn,
            "translate",
            Some(("builtin", "baidu", "百度翻译")),
            None,
            CallChars { prompt: 100, completion: 100 },
            true,
            None,
        )
        .expect("记账");
        let plan = prepare(&conn, "Hello", None).expect("另一家仍可用");
        assert_eq!(plan.candidates.len(), 1, "超额的一家被剔除");
        assert_eq!(plan.candidates[0].provider_id, "xfyun");
        // 任一家达到上限即触发提示（quotaExhausted），引导用户添加自定义服务商。
        assert!(builtin_any_exhausted(&conn).expect("超额提示判定"));

        // 两家都到上限 → 拒绝。
        conn.execute(
            "UPDATE builtin_translate_providers SET monthly_granted = 1 WHERE provider_id = 'xfyun'",
            [],
        )
        .expect("调低讯飞上限");
        crate::llm::quota::record_call(
            &conn,
            "translate",
            Some(("builtin", "xfyun", "讯飞翻译")),
            None,
            CallChars { prompt: 100, completion: 100 },
            true,
            None,
        )
        .expect("记账");
        assert!(prepare(&conn, "Hello", None).is_err(), "两家都用尽应拒绝");
    }

    #[test]
    fn prepare_falls_back_to_custom_baidu_credentials() {
        let (_dir, conn) = fresh_db();
        repo::save(&conn, true, "auto", "baidu", Some("my-key"), Some("my-appid"))
            .expect("保存自填凭据");
        let plan = prepare(&conn, "Hello", None).expect("自填百度凭据应可用");
        assert!(!plan.is_builtin, "自填凭据不计入内置额度");
        assert_eq!(plan.candidates[0].app_id, "my-appid");
        assert_eq!(plan.candidates[0].secret, "my-key");
    }

    #[test]
    fn prepare_prefers_first_usable_custom_provider() {
        let (_dir, conn) = fresh_db();
        custom_repo::insert(&conn, "我的 DeepL", "deepl", "", "key-1").expect("新增 1");
        custom_repo::insert(&conn, "我的 Google", "google", "", "key-2").expect("新增 2");
        let plan = prepare(&conn, "Hello", None).expect("自填服务商可用");
        assert_eq!(plan.candidates[0].provider_id, "deepl", "按 sort_order 取第一个");
        assert_eq!(plan.candidates[0].service_name, "我的 DeepL");
        assert_eq!(plan.candidates[0].secret, "key-1");
        assert!(!plan.is_builtin, "自填服务商不计入内置额度");
        assert_eq!(plan.candidates.len(), 1, "自填通道不做内置切换");
    }

    #[test]
    fn prepare_prefers_legacy_custom_column_over_builtin() {
        let (_dir, conn) = fresh_db();
        repo::save(&conn, true, "auto", "deepl", Some("legacy-key"), None)
            .expect("保存旧版自定义配置");
        let plan = prepare(&conn, "Hello", None).expect("旧配置可用");
        assert_eq!(plan.candidates[0].secret, "legacy-key");
        assert_eq!(plan.candidates[0].provider_id, "deepl");
        assert!(!plan.is_builtin);
    }

    #[test]
    fn prepare_falls_back_to_builtin_when_all_custom_are_disabled() {
        let (_dir, conn) = fresh_db();
        let id = custom_repo::insert(&conn, "停用", "deepl", "", "key-1").expect("新增");
        custom_repo::update(&conn, id, "停用", "deepl", "", 2).expect("停用");
        let plan = prepare(&conn, "Hello", None).expect("回退内置");
        assert!(plan.is_builtin, "自填全停用时回退内置服务");
        assert_eq!(plan.candidates[0].provider_id, "baidu");
        assert_eq!(plan.candidates[0].service_name, "百度翻译");
    }

    #[test]
    fn prepare_works_when_only_xfyun_is_enabled() {
        let (_dir, conn) = fresh_db();
        let rows = builtin_repo::list_all(&conn).expect("列表");
        let baidu = rows.iter().find(|r| r.provider_id == "baidu").expect("百度行");
        builtin_repo::update_config(&conn, baidu.id, false, baidu.monthly_granted)
            .expect("停用百度");
        let plan = prepare(&conn, "Hello", None).expect("只启用讯飞也应可用");
        assert!(plan.is_builtin);
        assert_eq!(plan.candidates.len(), 1, "只剩讯飞一家候选");
        assert_eq!(plan.candidates[0].provider_id, "xfyun");
        assert_eq!(plan.candidates[0].service_name, "讯飞翻译");
    }

    #[test]
    fn global_quota_must_not_block_translation_when_a_provider_still_has_grant() {
        let (_dir, conn) = fresh_db();
        // 主对话等其它场景消耗了全局额度，但两家内置翻译各自仍有独立额度。
        let period = current_period();
        conn.execute(
            "INSERT INTO quota_usage_monthly (period_key, granted, used, requests) \
             VALUES (?1, ?2, ?2, 1)",
            rusqlite::params![period, crate::config::QUOTA_MONTHLY_CREDITS],
        )
        .expect("模拟全局额度用尽");
        let plan = prepare(&conn, "Hello", None).expect("各家仍有额度时不应被全局总闸拦截");
        assert_eq!(plan.candidates.len(), 2);
    }
}

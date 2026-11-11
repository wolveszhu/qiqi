//! translate_settings 表访问（全局划词翻译配置，单行 id=1）。
//!
//! 红线（rule.md §H / 与 `search_settings.rs` 同口径）：内置凭据与自定义 Key 的明文
//! 仅在本层与命令层之间流转，**禁止进日志 / 错误 / DTO**；DTO 只回 `has_*` + 末 4 位指纹。
//!
//! M11 只建表：单行与内置凭据由 `seed_builtin` 在首次启动时幂等播种，
//! 明文不进仓库、不进迁移 SQL（发布守卫 INSERT 白名单 + 禁提交 API Key 红线）。

use rusqlite::{Connection, params};

use crate::error::AppError;

/// 翻译服务商：`Baidu` 为内置零配置通道，其余为用户自填 Key 的 BYOK 预设。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslateProvider {
    Baidu,
    Deepl,
    Youdao,
    Google,
}

impl TranslateProvider {
    /// 返回持久化到库内的服务商标识。
    pub fn as_str(self) -> &'static str {
        match self {
            TranslateProvider::Baidu => "baidu",
            TranslateProvider::Deepl => "deepl",
            TranslateProvider::Youdao => "youdao",
            TranslateProvider::Google => "google",
        }
    }

    /// 解析前端传入的服务商标识；非法值报 `AppError::Config`。
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        match raw {
            "baidu" => Ok(TranslateProvider::Baidu),
            "deepl" => Ok(TranslateProvider::Deepl),
            "youdao" => Ok(TranslateProvider::Youdao),
            "google" => Ok(TranslateProvider::Google),
            other => Err(AppError::Config(format!("未知翻译服务商: {other}"))),
        }
    }

    /// 是否为计入内置额度的零配置通道。
    pub fn is_builtin(self) -> bool {
        matches!(self, TranslateProvider::Baidu)
    }
}

/// 译入语言：本版仅中英互译，`Auto` 表示按源语言自动取相反语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLang {
    Auto,
    Zh,
    En,
}

impl TargetLang {
    /// 返回持久化到库内的语言标识。
    pub fn as_str(self) -> &'static str {
        match self {
            TargetLang::Auto => "auto",
            TargetLang::Zh => "zh",
            TargetLang::En => "en",
        }
    }

    /// 解析语言标识；非法值报 `AppError::Config`。
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        match raw {
            "auto" => Ok(TargetLang::Auto),
            "zh" => Ok(TargetLang::Zh),
            "en" => Ok(TargetLang::En),
            other => Err(AppError::Config(format!("不支持的译入语言: {other}"))),
        }
    }
}

/// 划词翻译配置行（含明文凭据，仅内部使用；DTO 不暴露）。
#[derive(Debug, Clone)]
pub struct TranslateSettingsRow {
    pub enabled: bool,
    pub target_lang: String,
    pub provider: String,
    pub custom_api_key: String,
    pub custom_app_id: String,
    pub builtin_app_id: String,
    pub builtin_secret: String,
}

impl TranslateSettingsRow {
    /// 编译期注入的内置百度凭据是否齐全。
    pub fn has_builtin_credential(&self) -> bool {
        !self.builtin_app_id.is_empty() && !self.builtin_secret.is_empty()
    }

    /// 用户是否配置了自定义服务的 Key。
    pub fn has_custom_key(&self) -> bool {
        !self.custom_api_key.is_empty()
    }

    /// 服务商枚举（未知值回落内置通道，避免脏数据导致启动失败）。
    pub fn provider_enum(&self) -> TranslateProvider {
        TranslateProvider::parse(&self.provider).unwrap_or(TranslateProvider::Baidu)
    }

    /// 译入语言枚举（未知值回落自动互译）。
    pub fn target_lang_enum(&self) -> TargetLang {
        TargetLang::parse(&self.target_lang).unwrap_or(TargetLang::Auto)
    }

    /// 是否具备发起翻译的条件：总开关开启，且内置凭据或自定义 Key 至少其一就绪。
    pub fn is_ready(&self) -> bool {
        self.enabled && (self.has_builtin_credential() || self.has_custom_key())
    }
}

/// 每次启动幂等播种：保证单行存在，并**无条件**同步构建期注入的内置凭据。
///
/// `app_id` / `secret` 来自 `config::BAIDU_TRANSLATE_*`（`option_env!`）。未注入时为 `None`
/// → 写入空串：既让发布基线行数确定、内置通道降级为「未配置」但不阻断启动，
/// 也把存量库里已轮换作废的旧明文清掉（旧实现只在注入时覆盖，会残留失效 Key）。
/// 只写内置两列，**不覆盖**用户的开关 / 译入语言 / 服务商 / 自定义 Key。
pub fn seed_builtin(
    conn: &Connection,
    app_id: Option<&str>,
    secret: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO translate_settings (id, enabled, target_lang, provider) \
         VALUES (1, 1, 'auto', 'baidu')",
        [],
    )
    .map_err(AppError::from)?;
    conn.execute(
        "UPDATE translate_settings SET builtin_app_id = ?1, builtin_secret = ?2 WHERE id = 1",
        params![app_id.unwrap_or(""), secret.unwrap_or("")],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 读取配置（单行 id=1；播种已确保存在）。
pub fn get(conn: &Connection) -> Result<TranslateSettingsRow, AppError> {
    let row = conn
        .query_row(
            "SELECT enabled, target_lang, provider, custom_api_key, custom_app_id, \
                    builtin_app_id, builtin_secret \
             FROM translate_settings WHERE id = 1",
            [],
            |r| {
                Ok(TranslateSettingsRow {
                    enabled: r.get::<_, i64>(0)? != 0,
                    target_lang: r.get(1)?,
                    provider: r.get(2)?,
                    custom_api_key: r.get(3)?,
                    custom_app_id: r.get(4)?,
                    builtin_app_id: r.get(5)?,
                    builtin_secret: r.get(6)?,
                })
            },
        )
        .map_err(AppError::from)?;
    Ok(row)
}

/// 保存配置（单行 id=1）。
///
/// `custom_api_key` / `custom_app_id` 为 `None` = 不改动存量；`Some("")` = 显式清空。
/// 服务商与译入语言先做薄层校验，非法值不落库。
pub fn save(
    conn: &Connection,
    enabled: bool,
    target_lang: &str,
    provider: &str,
    custom_api_key: Option<&str>,
    custom_app_id: Option<&str>,
) -> Result<(), AppError> {
    TargetLang::parse(target_lang)?;
    TranslateProvider::parse(provider)?;
    conn.execute(
        "UPDATE translate_settings SET enabled = ?1, target_lang = ?2, provider = ?3 WHERE id = 1",
        params![i64::from(enabled), target_lang, provider],
    )
    .map_err(AppError::from)?;
    if let Some(key) = custom_api_key {
        conn.execute(
            "UPDATE translate_settings SET custom_api_key = ?1 WHERE id = 1",
            params![key],
        )
        .map_err(AppError::from)?;
    }
    if let Some(id) = custom_app_id {
        conn.execute("UPDATE translate_settings SET custom_app_id = ?1 WHERE id = 1", params![id])
            .map_err(AppError::from)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn seed_builtin_is_idempotent_and_keeps_user_settings() {
        let (_dir, conn) = fresh_db();
        seed_builtin(&conn, Some("app-id-a"), Some("secret-a")).expect("播种");
        save(&conn, false, "en", "deepl", Some("custom-key"), Some("custom-id")).expect("用户配置");

        seed_builtin(&conn, Some("app-id-b"), Some("secret-b")).expect("重复播种");
        let row = get(&conn).expect("读取");
        assert_eq!(row.builtin_app_id, "app-id-b", "内置凭据应随轮换同步");
        assert_eq!(row.target_lang, "en", "不得覆盖用户译入语言");
        assert_eq!(row.provider, "deepl", "不得覆盖用户服务商");
        assert_eq!(row.custom_api_key, "custom-key", "不得覆盖用户 Key");
        assert!(!row.enabled, "不得覆盖用户总开关");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM translate_settings", [], |r| r.get(0))
            .expect("统计行数");
        assert_eq!(count, 1, "播种幂等，单行表不得出现第二行");
    }

    #[test]
    fn get_returns_seeded_defaults() {
        let (_dir, conn) = fresh_db();
        let row = get(&conn).expect("读取");
        assert!(row.enabled, "零配置通道默认开启");
        assert_eq!(row.target_lang, "auto");
        assert_eq!(row.provider, "baidu");
        assert!(!row.has_custom_key());
        assert!(row.has_builtin_credential(), "M12 种子应写入内置百度凭据");
        assert!(row.is_ready(), "内置凭据就绪即可发起翻译");
        assert_eq!(row.provider_enum(), TranslateProvider::Baidu);
        assert_eq!(row.target_lang_enum(), TargetLang::Auto);
    }

    #[test]
    fn save_updates_fields_and_key_semantics() {
        let (_dir, conn) = fresh_db();
        save(&conn, true, "zh", "youdao", Some("key-1234"), Some("id-1234")).expect("保存");
        let row = get(&conn).expect("读取");
        assert_eq!(row.target_lang, "zh");
        assert_eq!(row.provider, "youdao");
        assert!(row.has_custom_key());
        assert!(row.is_ready());

        save(&conn, true, "auto", "baidu", None, None).expect("不改动 Key");
        let row = get(&conn).expect("读取");
        assert_eq!(row.custom_api_key, "key-1234", "None 不应清空 Key");
        assert_eq!(row.custom_app_id, "id-1234");

        save(&conn, true, "auto", "baidu", Some(""), Some("")).expect("显式清空");
        assert!(!get(&conn).expect("读取").has_custom_key());
    }

    #[test]
    fn save_rejects_unknown_provider_or_language() {
        let (_dir, conn) = fresh_db();
        assert!(save(&conn, true, "auto", "bing", None, None).is_err(), "未知服务商应报错");
        assert!(save(&conn, true, "ja", "baidu", None, None).is_err(), "非中英语种应报错");
    }

    #[test]
    fn provider_enum_flags_builtin_channel() {
        assert!(TranslateProvider::Baidu.is_builtin());
        assert!(!TranslateProvider::Deepl.is_builtin());
        assert_eq!(TranslateProvider::Google.as_str(), "google");
        assert_eq!(TargetLang::En.as_str(), "en");
    }
}

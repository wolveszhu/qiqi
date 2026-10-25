//! search_settings 表访问（联网搜索服务 BYOK 配置，单一行 id=1）。
//!
//! 红线（rule.md §H / 与 `settings.rs` 同口径）：`api_key` 明文仅在本层与命令层之间流转，
//! **禁止进日志/错误/DTO**；DTO 只回 `has_key` + 末 4 位指纹。

use rusqlite::Connection;

use crate::error::AppError;

/// 搜索服务商（首期实现：tavily / brave / serpapi）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchProvider {
    Tavily,
    Brave,
    Serpapi,
}

impl SearchProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            SearchProvider::Tavily => "tavily",
            SearchProvider::Brave => "brave",
            SearchProvider::Serpapi => "serpapi",
        }
    }
    /// 解析前端传入的 provider 字符串（薄层参数校验，非法值报错）。
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        match raw {
            "tavily" => Ok(SearchProvider::Tavily),
            "brave" => Ok(SearchProvider::Brave),
            "serpapi" => Ok(SearchProvider::Serpapi),
            other => Err(AppError::Config(format!("未知搜索服务商: {other}"))),
        }
    }
}

/// 搜索配置行（`api_key` 为明文，仅内部使用；DTO 不暴露）。
#[derive(Debug, Clone)]
pub struct SearchSettingsRow {
    pub provider: String,
    pub api_key: String,
    pub enabled: bool,
    pub max_results: i64,
    pub lang: String,
}

impl SearchSettingsRow {
    pub fn has_key(&self) -> bool {
        !self.api_key.is_empty()
    }
    /// 是否具备发起搜索的条件：已启用且有 Key。
    pub fn is_ready(&self) -> bool {
        self.enabled && self.has_key()
    }
    pub fn provider_enum(&self) -> SearchProvider {
        SearchProvider::parse(&self.provider).unwrap_or(SearchProvider::Tavily)
    }
}

/// 读取搜索配置（单行 id=1；迁移已确保存在）。
pub fn get(conn: &Connection) -> Result<SearchSettingsRow, AppError> {
    let row = conn
        .query_row(
            "SELECT provider, api_key, enabled, max_results, lang FROM search_settings WHERE id = 1",
            [],
            |r| {
                Ok(SearchSettingsRow {
                    provider: r.get(0)?,
                    api_key: r.get(1)?,
                    enabled: r.get::<_, i64>(2)? != 0,
                    max_results: r.get(3)?,
                    lang: r.get(4)?,
                })
            },
        )
        .map_err(AppError::from)?;
    Ok(row)
}

/// 保存搜索配置（upsert id=1）。
///
/// `api_key` 为 `None` = 不改动存量 Key；`Some("")` = 显式清空；`Some(k)` = 覆盖。
/// `max_results` 夹取到 [1, 10]；provider 非法直接报错（薄层校验）。
pub fn save(
    conn: &Connection,
    provider: &str,
    api_key: Option<&str>,
    enabled: bool,
    max_results: i64,
    lang: &str,
) -> Result<(), AppError> {
    SearchProvider::parse(provider)?; // 薄层校验
    let max_results = max_results.clamp(1, 10);
    // 两 UPDATE 顺序执行（单行 id=1，幂等），与现有仓储风格一致，避免事务借用 &mut。
    conn.execute(
        "UPDATE search_settings SET provider = ?1, enabled = ?2, max_results = ?3, lang = ?4 \
         WHERE id = 1",
        rusqlite::params![provider, i64::from(enabled), max_results, lang],
    )
    .map_err(AppError::from)?;
    if let Some(key) = api_key {
        conn.execute(
            "UPDATE search_settings SET api_key = ?1 WHERE id = 1",
            rusqlite::params![key],
        )
        .map_err(AppError::from)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn get_returns_seeded_default() {
        let (_dir, conn) = fresh_db();
        let row = get(&conn).expect("读取");
        assert_eq!(row.provider, "tavily");
        assert!(!row.enabled);
        assert!(!row.has_key());
        assert!(!row.is_ready());
    }

    #[test]
    fn save_updates_fields_and_key_semantics() {
        let (_dir, conn) = fresh_db();
        save(&conn, "brave", Some("sk-brave-7788"), true, 8, "en").expect("保存");
        let row = get(&conn).expect("读取");
        assert_eq!(row.provider, "brave");
        assert!(row.enabled);
        assert!(row.has_key());
        assert!(row.is_ready());
        assert_eq!(row.max_results, 8);

        // None = 不改动 Key
        save(&conn, "tavily", None, false, 5, "zh").expect("保存");
        let row = get(&conn).expect("读取");
        assert_eq!(row.provider, "tavily");
        assert!(!row.enabled);
        assert!(row.has_key(), "None 不应清空 Key");
        assert_eq!(row.max_results, 5);

        // 空串 = 清空
        save(&conn, "tavily", Some(""), false, 5, "zh").expect("清空");
        assert!(!get(&conn).expect("读取").has_key());
    }

    #[test]
    fn save_clamps_max_results() {
        let (_dir, conn) = fresh_db();
        save(&conn, "tavily", Some("k"), true, 99, "zh").expect("保存");
        assert_eq!(get(&conn).expect("读取").max_results, 10, "应夹取到上界");
        save(&conn, "tavily", Some("k"), true, 0, "zh").expect("保存");
        assert_eq!(get(&conn).expect("读取").max_results, 1, "应夹取到下界");
    }

    #[test]
    fn save_rejects_unknown_provider() {
        let (_dir, conn) = fresh_db();
        assert!(save(&conn, "bing", None, true, 5, "zh").is_err(), "未知服务商应报错");
    }
}

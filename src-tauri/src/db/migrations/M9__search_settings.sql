-- M9：联网搜索服务配置（BYOK；用户自填搜索 API Key）。
--
-- 单一配置行（id 固定 = 1，CHECK 约束保证全局只有一行）：
-- - provider：搜索服务商（tavily / brave / serpapi；首期实现三者）；
-- - api_key：明文仅在本层与命令层之间流转，库已全库 SQLCipher 加密，接口不回明文；
-- - enabled：是否启用联网搜索（0/1）；
-- - max_results：单次搜索结果上限；
-- - lang：结果语言偏好（zh/en）。
--
-- 幂等：CREATE TABLE IF NOT EXISTS + 仅当缺失时插入默认行（避免重复种子）。

CREATE TABLE IF NOT EXISTS search_settings (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    provider    TEXT    NOT NULL DEFAULT 'tavily',
    api_key     TEXT    NOT NULL DEFAULT '',
    enabled     INTEGER NOT NULL DEFAULT 0,
    max_results INTEGER NOT NULL DEFAULT 5,
    lang        TEXT    NOT NULL DEFAULT 'zh'
);

INSERT INTO search_settings (id, provider, api_key, enabled, max_results, lang)
SELECT 1, 'tavily', '', 0, 5, 'zh'
WHERE NOT EXISTS (SELECT 1 FROM search_settings WHERE id = 1);

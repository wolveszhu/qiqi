-- M12：内置百度翻译凭据种子 + 用户自填多服务商表。
--
-- 依据：用户决策 2026-11-09「内置凭据写入数据表」「可自填多个服务商，自填优先」。
-- 与 M6 同口径：内置共享凭据属**已登记的生产凭据**，只允许存在于 Rust 侧迁移 SQL，
-- 不得进入前端产物 / dist；上线前须在百度开放平台配置额度上限与调用白名单。
-- 迁移按版本号执行且不校验内容，存量库不会重跑 M11，故种子与表结构一律追加本迁移，
-- 禁止改写 M11 历史 SQL（development-guide §4 / baseline §E）。
-- 单事务幂等：NOT EXISTS 守卫 + 条件 UPDATE，重复执行不重复插入、不覆盖用户已改配置。

-- 1) 单行配置不存在时插入内置百度凭据（全新空库路径）。
INSERT INTO translate_settings (id, enabled, target_lang, provider, builtin_app_id, builtin_secret)
SELECT 1, 1, 'auto', 'baidu', '20261107002714302', 'YjDAPNYWWfN7STvnQjky'
WHERE NOT EXISTS (SELECT 1 FROM translate_settings WHERE id = 1);

-- 2) 存量库已有空凭据行时补齐（不动用户已填的自定义列、开关与译入语言）。
UPDATE translate_settings
SET builtin_app_id = '20261107002714302', builtin_secret = 'YjDAPNYWWfN7STvnQjky'
WHERE id = 1 AND (builtin_app_id = '' OR builtin_secret = '');

-- 3) 用户自填的多个翻译服务商（与 custom_model_settings 对称；无种子，用户自配）。
CREATE TABLE IF NOT EXISTS custom_translate_providers (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        VARCHAR(20) NOT NULL,      -- 展示名（用户自取，如「我的 DeepL」）
    provider    TEXT NOT NULL,             -- baidu / deepl / youdao / google
    app_id      TEXT NOT NULL DEFAULT '', -- 应用 ID（百度 / 有道需要，其余留空）
    api_key     TEXT NOT NULL DEFAULT '', -- 明文仅在 SQLCipher 库内流转，接口只回末 4 位
    is_can_use  INTEGER DEFAULT 1,        -- 1=启用 / 2=停用（停用不删，保配置）
    sort_order  INTEGER DEFAULT 0         -- 越小越前；自填优先按此顺序取第一个可用
);

CREATE INDEX IF NOT EXISTS idx_custom_translate_provider_order
    ON custom_translate_providers (sort_order, id);

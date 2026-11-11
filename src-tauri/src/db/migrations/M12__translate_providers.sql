-- M12：内置百度翻译凭据种子 + 用户自填多服务商表。
--
-- 依据：用户决策 2026-11-09「内置凭据写入数据表」「可自填多个服务商，自填优先」。
-- **凭据边界（2026-11-11 修订）**：builtin_app_id / builtin_secret 一律为空占位；
-- 真实凭据只由构建期环境变量 QIQI_BAIDU_APPID / QIQI_BAIDU_SECRET 注入，
-- 经启动路径 `db::repositories::builtin_credentials::sync` 写入 SQLCipher 库。
-- 迁移 SQL 随仓库分发且被 include_str! 编入二进制，绝不允许出现明文凭据。
-- 迁移按版本号执行且不校验内容，存量库不会重跑 M11，故种子与表结构一律追加本迁移，
-- 禁止改写 M11 历史 SQL（development-guide §4 / baseline §E）。
-- 单事务幂等：NOT EXISTS 守卫 + 条件 UPDATE，重复执行不重复插入、不覆盖用户已改配置。

-- 1) 单行配置不存在时插入空占位（全新空库路径；真实凭据由启动播种写入）。
INSERT INTO translate_settings (id, enabled, target_lang, provider, builtin_app_id, builtin_secret)
SELECT 1, 1, 'auto', 'baidu', '', ''
WHERE NOT EXISTS (SELECT 1 FROM translate_settings WHERE id = 1);

-- 2) 存量库已有空凭据行时补齐空占位（不动用户已填的自定义列、开关与译入语言）。
--    保留语句结构以维持行数与幂等语义；凭据列恒为空，由启动播种统一接管。
UPDATE translate_settings
SET builtin_app_id = '', builtin_secret = ''
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

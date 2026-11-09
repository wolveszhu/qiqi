-- M11：全局划词翻译配置（单行 id=1，只建表、不写种子）。
--
-- 设计依据：.codebuddy/plans/全局划词翻译-悬浮工具条与译文浮窗实施计划.md
-- - 内置百度翻译 APPID / 密钥由构建期环境变量注入，首次启动由 Rust 参数化幂等播种，
--   故本 SQL 不含任何凭据字面值，也不含写入语句（发布守卫白名单 + 禁提交 Key 红线）。
-- - 自定义通道（DeepL / 有道 / Google）的 Key 由用户在设置面板填写，接口只回脱敏指纹。
-- - target_lang：auto = 中英自动互译；zh / en = 锁定译入语言（本版仅中英互译）。
-- - provider：baidu（内置零配置）/ deepl / youdao / google（BYOK）。
-- - enabled：划词翻译总开关，默认值供运行时播种使用。
--
-- 幂等：CREATE TABLE IF NOT EXISTS；默认值只表达结构，不产生任何行。

CREATE TABLE IF NOT EXISTS translate_settings (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    enabled         INTEGER NOT NULL DEFAULT 1,
    target_lang     TEXT    NOT NULL DEFAULT 'auto',
    provider        TEXT    NOT NULL DEFAULT 'baidu',
    custom_api_key  TEXT    NOT NULL DEFAULT '',
    custom_app_id   TEXT    NOT NULL DEFAULT '',
    builtin_app_id  TEXT    NOT NULL DEFAULT '',
    builtin_secret  TEXT    NOT NULL DEFAULT ''
);

-- M11：全局划词翻译配置（单行 id=1，只建表、不写种子）。
--
-- 设计依据：.codebuddy/plans/全局划词翻译-悬浮工具条与译文浮窗实施计划.md
-- - 本迁移只建表、不写种子：内置百度凭据与用户自填多服务商表见 M12。
--   迁移按版本号执行且不校验内容，存量库不会重跑本文件，故后续数据变更一律追加新迁移。
-- - 构建期环境变量仍可覆盖内置凭据（便于轮换），但不注入时 M12 种子即可开箱可用。
-- - 自定义通道（DeepL / 有道 / Google）的 Key 由用户在设置面板填写，接口只回脱敏指纹。
-- - target_lang：auto = 中英自动互译；zh / en = 锁定译入语言（本版仅中英互译）。
-- - provider：baidu（内置零配置）/ deepl / youdao / google（BYOK）。
-- - enabled：划词翻译总开关，默认值供运行时播种使用。
--
-- 幂等：CREATE TABLE IF NOT EXISTS；默认值只表达结构，不产生任何行。

CREATE TABLE IF NOT EXISTS translate_settings (
    id              INTEGER PRIMARY KEY CHECK (id = 1), -- 单行配置：全表仅允许 id=1 一行
    enabled         INTEGER NOT NULL DEFAULT 1,         -- 总开关：1=启用划词工具条，0=不弹出
    target_lang     TEXT    NOT NULL DEFAULT 'auto',    -- 译入语言：auto 自动互译 / zh 译入中文 / en 译入英文
    provider        TEXT    NOT NULL DEFAULT 'baidu',   -- 服务商：baidu 内置零配置 / deepl / youdao / google
    custom_api_key  TEXT    NOT NULL DEFAULT '',        -- 用户自填密钥（BYOK）；读取接口只回末 4 位指纹
    custom_app_id   TEXT    NOT NULL DEFAULT '',        -- 用户自填应用 ID（百度 / 有道需要，其余服务留空）
    builtin_app_id  TEXT    NOT NULL DEFAULT '',        -- 内置百度 APPID：编译期注入后由 Rust 幂等播种
    builtin_secret  TEXT    NOT NULL DEFAULT ''         -- 内置百度密钥：同上，明文不进仓库与迁移脚本
);

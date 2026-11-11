-- M6：default_model_settings 内置免费额度模型（智谱 / 硅基流动）。
--
-- 背景：本表最初加在 M1 里，但 rusqlite_migration **按版本号执行、不校验内容**——
-- 已执行过 M1 的存量库不会重跑，表会静默缺失。按 development-guide §4
-- 「结构变更必须追加新迁移、禁止改历史 SQL」的规则移至本独立迁移。
--
-- 单事务幂等：IF NOT EXISTS 建表 + NOT EXISTS 守卫种子（重跑不重复插入）。
--
-- **凭据边界（2026-11-11）**：两行 api_key 一律为空占位。内置额度模型的真实 Key
-- 只由构建期环境变量（QIQI_ZHIPU_API_KEY / QIQI_SILICONFLOW_API_KEY）注入，
-- 经 `db::repositories::builtin_credentials::sync` 在每次启动时写入 SQLCipher 库。
-- 迁移 SQL 会被 include_str! 编入二进制且随仓库分发，绝不允许出现明文凭据；
-- 本文件只调整种子值，表结构 / 行数 / 幂等守卫均不变（存量库不会重跑）。

CREATE TABLE IF NOT EXISTS default_model_settings (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    name                 VARCHAR(20) NOT NULL,      -- 平台名称（智谱/硅基流动）
    type                 INTEGER DEFAULT 1,         -- 1=LLM, 2=语音,3=图像,4=视频,5=翻译
    base_url             VARCHAR(255) NOT NULL,     -- 模型 URL(API请求地址)
    models               JSON NOT NULL,             -- 模型列表
    api_key              VARCHAR(255) NOT NULL,     -- 模型/平台 API 密钥
    is_can_use           INTEGER DEFAULT 1          -- 1=可用, 2=不可用
);

CREATE INDEX IF NOT EXISTS ids_model_setting_name_type ON default_model_settings (name, type);

INSERT INTO default_model_settings (name, type, base_url, models, api_key, is_can_use)
SELECT  '智谱',1,  'https://open.bigmodel.cn/api/paas/v4', '["glm-4-flash"]', '', 1
WHERE NOT EXISTS (SELECT 1 FROM default_model_settings WHERE name = '智谱');

INSERT INTO default_model_settings (name, type, base_url, models, api_key, is_can_use)
SELECT  '硅基流动',  1,'https://api.siliconflow.cn/v1', '["Qwen/Qwen2.5-7B-Instruct", "THUDM/GLM-4-9B-0414", "THUDM/GLM-Z1-9B-0414", "Qwen/Qwen3-8B", "deepseek-ai/DeepSeek-R1-0528-Qwen3-8B", "Qwen/Qwen3.5-4B"]', '', 1
WHERE NOT EXISTS (SELECT 1 FROM default_model_settings WHERE name = '硅基流动');

-- 额度默认口径对齐（用户原在 M1 DDL 将 default 1→2 的意图）：
-- M1 种子行不带该列、走 DDL 默认 1，故在此把存量仍为 1 的 type=1 行切到 2（BYOK 默认）。
-- 用户随后可在设置面板「使用内置额度」随时切回 1（启用内置模型兜底）。
UPDATE app_settings SET is_custom_quota_used = 2 WHERE type = 1 AND is_custom_quota_used = 1;

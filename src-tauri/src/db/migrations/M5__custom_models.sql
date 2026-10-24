-- 自定义大模型表（与 default_model_settings 对称；用户自配 Key，优先于内置）。
--
-- 依据：用户决策 2026-10-24「自定义优先，内置兜底」——app_settings 三档之外的
-- **多模型**来源，故新建对称表而非塞 JSON（多一套仓储换清晰边界）。
--
-- 与内置表差异：多 `sort_order`（前端列表排序用；内置表只读无排序需求）。
-- 单事务幂等：IF NOT EXISTS 建表；无种子数据（用户自配）。

CREATE TABLE IF NOT EXISTS custom_model_settings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        VARCHAR(20) NOT NULL,      -- 展示名（用户自取，如"我的 DeepSeek"）
    type        INTEGER DEFAULT 1,         -- 1=LLM, 2=语音,3=图像,4=视频,5=翻译（与内置表同义）
    base_url    VARCHAR(255) NOT NULL,     -- OpenAI 兼容 API 地址
    models      JSON NOT NULL,             -- 模型名数组，如 ["deepseek-chat","deepseek-reasoner"]
    api_key     VARCHAR(255) NOT NULL,     -- 明文仅在 SQLCipher 库内流转
    is_can_use  INTEGER DEFAULT 1,         -- 1=可用, 2=不可用（停用不删，保配置）
    sort_order  INTEGER DEFAULT 0          -- 列表排序（越小越前；新增追加到尾）
);

CREATE INDEX IF NOT EXISTS idx_custom_model_name_type ON custom_model_settings (name, type);

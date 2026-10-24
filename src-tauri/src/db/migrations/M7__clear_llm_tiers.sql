-- M7：清空 app_settings(type=1) 的三档 LLM 配置（用户决策：三档 UI 合并为单一
-- 「大模型服务商」平台列表，链路改由 custom_model_settings 驱动）。
--
-- 语义：
-- - 只把 value 重置为空三档结构（main/memory/fallback 全空）；
-- - **保留行本身与 is_custom_quota_used**（额度模式是独立开关，不属于三档配置）；
-- - 幂等：已是空三档的库重复执行无副作用（UPDATE 同值）。

UPDATE app_settings
SET value = '{"main":{"baseUrl":"","model":"","apiKey":"","modelCustom":""},"memory":{"baseUrl":"","model":"","apiKey":"","modelCustom":""},"fallback":{"baseUrl":"","model":"","apiKey":"","modelCustom":""}}'
WHERE type = 1;

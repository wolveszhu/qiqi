-- 勘误 #7 数据迁移：app_settings(type=1) 的扁平单档结构 → 三档 tiered 结构。
--
-- 背景：B4 执行时的 M1 种子为扁平结构（{"llm_base_url":...}），与 B5 契约的
-- `SettingsSnapshot`（main/memory/fallback 三档）不匹配。执行时点已有旧库，
-- 按计划裁决走数据迁移分支（而非改已执行的历史迁移）。
--
-- 幂等：以 json_type 判定结构，已是三档（含 "main" 键）的库重跑不生效。
-- 语义：扁平结构整体迁移为 main 档（存量用户的主对话配置），memory/fallback 留空，
--       用户在设置面板（D2）按选型方案补配；未配置的档位由 C4 router 回落链兜底。

UPDATE app_settings
SET value = json_patch(
  '{}',
  json_object(
    'main', json_object(
      'baseUrl', COALESCE(json_extract(value, '$.llm_base_url'), ''),
      'model', COALESCE(json_extract(value, '$.llm_model'), ''),
      'apiKey', COALESCE(json_extract(value, '$.llm_api_key'), ''),
      'modelCustom', COALESCE(json_extract(value, '$.llm_model_custom'), '')
    ),
    'memory', json_object('baseUrl', '', 'model', '', 'apiKey', '', 'modelCustom', ''),
    'fallback', json_object('baseUrl', '', 'model', '', 'apiKey', '', 'modelCustom', '')
  )
)
WHERE type = 1
  AND value IS NOT NULL
  AND json_type(value) = 'object'
  AND json_extract(value, '$.main') IS NULL;

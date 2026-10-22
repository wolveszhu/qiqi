-- M4 数据迁移（F4 / 勘误 #7 同模式）：app_settings(type=2) 旧单段勿扰格式 →
-- 多段 + 主动频率档位新格式 `{"dnd":{...},"frequency":...}`。
--
-- 背景：M1 种子写入的 type=2 为勘误 #13 前的旧单段结构
-- （{"dnd_enabled":false,"dnd_start_time":"09:00","dnd_end_time":"18:00"}），
-- 而 B5/D2 契约为新结构。此前仅靠读取端 LegacyDndValue 兼容兜底，导致**新库
-- frequency 持久化为空串**（后端语义脏值；前端有默认兜底故主链路无感）。
-- 本迁移在链上必经执行，使新库种子即为规范结构，读取兼容降级为纯防御路径。
--
-- 幂等：以 json_type + `$.dnd` 键判定，已是新结构（或含 dnd 键）的库重跑不生效。
-- 语义：旧单段整体迁为一段 ranges；frequency 无旧值可迁，按 D2.3 默认档 "normal" 落值。

UPDATE app_settings
SET value = json_patch(
  '{}',
  json_object(
    'dnd', json_object(
      'enabled', CASE
        WHEN COALESCE(json_extract(value, '$.dnd_enabled'), 0) THEN json('true')
        ELSE json('false')
      END,
      'ranges', CASE
        WHEN COALESCE(json_extract(value, '$.dnd_start_time'), '') = ''
          OR COALESCE(json_extract(value, '$.dnd_end_time'), '') = ''
        THEN json('[]')
        ELSE json_array(json_object(
          'start', json_extract(value, '$.dnd_start_time'),
          'end', json_extract(value, '$.dnd_end_time')
        ))
      END
    ),
    'frequency', 'normal'
  )
)
WHERE type = 2
  AND value IS NOT NULL
  AND json_type(value) = 'object'
  AND json_extract(value, '$.dnd') IS NULL;

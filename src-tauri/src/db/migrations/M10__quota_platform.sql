-- M10：额度用量增加「模型所属平台」维度 + 月度额度调整为 100,000。
--
-- 背景（用户决策 2026-11-06）：
-- - quota_usage 原只记 model，无法区分同一模型在哪个平台被调用；
-- - quota_usage_monthly 原以 period_key 单主键，只存全局用量，无法按平台拆分；
-- - 内置月度额度由 200,000 调整为 100,000 credits。
--
-- 结构决策：**不动 M8 历史 SQL**（迁移铁律），改为加量迁移：
-- 1. quota_usage 加列 platform（默认 'unknown'，兼容存量行）；
-- 2. 新建 quota_usage_platform_monthly(period_key, platform) 复合主键，
--    原 quota_usage_monthly 仍作全局汇总（granted/used/requests 语义不变）；
-- 3. 存量明细按 model 反查 default_model_settings.models（JSON 数组）回填平台名，
--    查不到填 'unknown'；存量平台月表由明细聚合回填；
-- 4. 存量汇总行的 granted 从旧默认值 200,000 更新为 100,000
--    （只改等于旧默认值的行，保留可能的后台覆盖值）。
--
-- 记账范围不变：仅内置额度段（tier='builtin'）落库，自定义 BYOK 不落库。
-- 幂等：所有 UPDATE 带条件、聚合回填用 ON CONFLICT；对新装空库不产生任何行。

ALTER TABLE quota_usage
    ADD COLUMN platform TEXT NOT NULL DEFAULT 'unknown';  -- 平台名（智谱 / 硅基流动 / unknown）

CREATE INDEX IF NOT EXISTS idx_quota_usage_platform ON quota_usage (period_key, platform);

CREATE TABLE IF NOT EXISTS quota_usage_platform_monthly (
    period_key  TEXT    NOT NULL,                    -- 本地自然月 YYYY-MM
    platform    TEXT    NOT NULL,                    -- default_model_settings.name
    used        INTEGER NOT NULL DEFAULT 0,         -- 该平台当月已用 credits
    requests    INTEGER NOT NULL DEFAULT 0,         -- 该平台当月记账请求次数
    updated_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (period_key, platform)
);

CREATE INDEX IF NOT EXISTS idx_quota_platform_month ON quota_usage_platform_monthly (period_key);

-- 存量明细回填：models 为 JSON 数组字符串，用 json_each 展开逐个匹配 model。
-- platform='unknown' 过滤保证重复执行不会覆盖已回填的真实平台名。
UPDATE quota_usage
SET platform = COALESCE(
        (SELECT d.name
           FROM default_model_settings d, json_each(d.models) j
          WHERE j.value = quota_usage.model
          LIMIT 1),
        'unknown')
WHERE platform = 'unknown';

-- 存量平台月表回填：从明细聚合（失败条目 credits=0 仍计入 requests，与全局汇总同口径）。
INSERT INTO quota_usage_platform_monthly (period_key, platform, used, requests)
SELECT period_key, platform, SUM(credits), COUNT(*)
  FROM quota_usage
 WHERE tier = 'builtin'
 GROUP BY period_key, platform
ON CONFLICT(period_key, platform) DO UPDATE SET
    used = excluded.used,
    requests = excluded.requests,
    updated_at = CURRENT_TIMESTAMP;

-- 旧默认额度对齐新值（只命中等于旧默认值的行，幂等）。
UPDATE quota_usage_monthly SET granted = 100000 WHERE granted = 200000;

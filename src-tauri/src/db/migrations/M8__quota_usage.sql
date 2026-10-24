-- M8：内置体验额度用量明细 + 月度汇总缓存。
--
-- 语义：
-- - **只记录内置额度消耗**（tier='builtin'）；自定义 BYOK（tier='custom'）**不落库、不展示**；
-- - 单笔 = 一次**真实发起**的 LLM 请求（failover 过程中每次尝试各记一条）；
-- - 失败/超时/熔断：credits=0 但保留明细（ok=0 + error），用于排查与审计；
-- - 计量单位 credits = ceil(prompt_tokens × W_in + completion_tokens × W_out)，
--   权重表集中在 config.rs（相对基准模型，不写死价格，便于后续接入收费模型）；
-- - period_key = 本地自然月 YYYY-MM，每月 1 日 00:00（本地）重置，不结转。
--
-- 幂等：CREATE TABLE IF NOT EXISTS + CREATE INDEX IF NOT EXISTS。

CREATE TABLE IF NOT EXISTS quota_usage (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    period_key       TEXT    NOT NULL,            -- 本地自然月 YYYY-MM
    scene            TEXT    NOT NULL,            -- chat / proactive / distill / diary
    tier             TEXT    NOT NULL,            -- builtin（记账）/ custom（不落库）
    model            TEXT    NOT NULL,
    prompt_tokens    INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens     INTEGER NOT NULL DEFAULT 0,
    credits          INTEGER NOT NULL DEFAULT 0,  -- 计权后扣减点数
    token_source     TEXT    NOT NULL DEFAULT 'api', -- api / estimated
    ok               INTEGER NOT NULL DEFAULT 1,  -- 1 成功 / 0 失败
    error            TEXT,                       -- 失败原因（成功为空）
    created_at       TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_quota_usage_period ON quota_usage (period_key, created_at DESC);

-- 月度汇总缓存：与明细写入**同事务**增量 upsert，UI 读取免全表 SUM。
CREATE TABLE IF NOT EXISTS quota_usage_monthly (
    period_key       TEXT PRIMARY KEY,       -- YYYY-MM
    granted          INTEGER NOT NULL,       -- 当月发放额度（credits）
    used             INTEGER NOT NULL DEFAULT 0,
    requests         INTEGER NOT NULL DEFAULT 0,
    updated_at       TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

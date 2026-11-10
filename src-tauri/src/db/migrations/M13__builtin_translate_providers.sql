-- M13：内置翻译服务商（百度 + 讯飞）凭据、独立月度额度与协同选路配置。
--
-- 依据：用户决策 2026-11-10「讯飞翻译与百度翻译一同作为内置服务，按当月用量选最少的一家，
-- 各自独立月度上限且设置页可改；凭据写入数据表」。
-- 与 M6 / M12 同口径：内置共享凭据属**已登记的生产凭据**，只允许存在于 Rust 侧迁移 SQL，
-- 不得进入前端产物 / dist；上线前须分别在百度、讯飞控制台配置额度上限与调用白名单。
-- 铁律：不改 M11 / M12 历史 SQL（存量库不会重跑），新表与种子一律追加本迁移。
-- 单事务幂等：CREATE TABLE IF NOT EXISTS + NOT EXISTS 守卫，重复执行不重复插入。

-- 1) 内置服务商表：一家一行。name 同时是额度记账的 platform 键，
--    沿用「百度翻译」这一取值以兼容 M12 起已产生的存量用量明细。
CREATE TABLE IF NOT EXISTS builtin_translate_providers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id     TEXT NOT NULL UNIQUE,      -- baidu / xfyun（对应 provider_for 的标识）
    name            TEXT NOT NULL,             -- 展示名，也是 quota_usage_platform_monthly.platform
    app_id          TEXT NOT NULL DEFAULT '', -- APPID
    api_secret      TEXT NOT NULL DEFAULT '', -- 百度密钥 / 讯飞 APISecret（HMAC 签名密钥）
    api_key         TEXT NOT NULL DEFAULT '', -- 讯飞 APIKey（百度留空）
    monthly_granted INTEGER NOT NULL DEFAULT 50000, -- 各自独立月度上限（credits），设置页可改
    is_can_use      INTEGER NOT NULL DEFAULT 1,     -- 1=启用 / 2=停用
    sort_order      INTEGER NOT NULL DEFAULT 0      -- 当月用量相同时的稳定次序
);

-- 2) 百度翻译：沿用 M12 已登记的凭据，保证存量库升级后两家都在表里。
INSERT INTO builtin_translate_providers
    (provider_id, name, app_id, api_secret, api_key, monthly_granted, is_can_use, sort_order)
SELECT 'baidu', '百度翻译', '20261107002714302', 'YjDAPNYWWfN7STvnQjky', '', 50000, 1, 0
WHERE NOT EXISTS (SELECT 1 FROM builtin_translate_providers WHERE provider_id = 'baidu');

-- 3) 讯飞机器翻译（新）：APPID / APISecret / APIKey 三项凭据。
INSERT INTO builtin_translate_providers
    (provider_id, name, app_id, api_secret, api_key, monthly_granted, is_can_use, sort_order)
SELECT 'xfyun', '讯飞翻译', 'c9955e0e', 'ZmFjNGJkNDE1M2RlYjk1ZjIwYmVkNDg4',
       'b8cbf3e6d624f59e38f39b6b321b65f8', 50000, 1, 1
WHERE NOT EXISTS (SELECT 1 FROM builtin_translate_providers WHERE provider_id = 'xfyun');

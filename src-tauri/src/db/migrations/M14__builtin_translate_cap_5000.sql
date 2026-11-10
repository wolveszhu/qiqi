-- M14：内置翻译兜底上限对齐每家每月 5,000（credits）。
--
-- 依据：用户决策 2026-11-10「底层保留上限作为兜底，修改为每月 5,000，
-- 超过之后提示用户提供自定义翻译服务商」。均衡仍靠按当月用量自动选最少的一家，
-- 本上限只是防单家被耗尽 / 被盗刷的最后闸门；达到上限的服务退出本轮候选，
-- 全部达到上限时前端按 `AppError::Config` 提示添加自定义翻译服务商。
-- 铁律：不改 M13 历史 SQL（存量库不会重跑），对齐一律追加本迁移。
-- 幂等：重复执行结果相同（全表对齐为同一常量，与 `config::BUILTIN_TRANSLATE_MONTHLY_GRANTED` 一致）。

UPDATE builtin_translate_providers SET monthly_granted = 5000;

//! quota_usage / quota_usage_monthly 表访问（内置体验额度用量明细与月度汇总）。
//!
//! 记账口径（方案见 `docs/billing/内置体验额度方案.md`）：
//! - **仅内置额度记账**：命中链段 `tier='builtin'` 才写明细；自定义 BYOK（`tier='custom'`）**不落库**；
//! - 单笔 = 一次**真实发起**的 LLM 请求（failover 过程中每次尝试各记一条）；
//! - 失败/超时/熔断：**扣 0 点**但保留明细（`ok=0` + `error`），便于排查与审计；
//! - 计量单位 **credits**：`ceil(prompt × W_in + completion × W_out)`，权重集中于 `config.rs`。
//!
//! 红线（rule.md §H）：全部参数化 SQL；错误一律 `AppError::Db`。

use rusqlite::{Connection, params};

use crate::config::{
    QUOTA_MONTHLY_CREDITS, TOKEN_ESTIMATE_CHARS_PER_TOKEN, WEIGHT_IN_DEFAULT, WEIGHT_OUT_DEFAULT,
};
use crate::db::models::{QuotaUsageDetailRow, QuotaUsageMonthlyRow};
use crate::error::AppError;

/// 月度周期键：本地时区自然月，格式 `YYYY-MM`。
pub fn period_key_of(date: &str) -> String {
    // date 形如 2026-10-25；取前 7 位即年月
    if date.len() >= 7 { date[..7].to_string() } else { date.to_string() }
}

/// 额度点换算：输入/输出分开计权后向上取整（保证"用了就算 1 点"）。
pub fn credits_of(prompt_tokens: i64, completion_tokens: i64) -> i64 {
    let raw = (prompt_tokens as f64) * WEIGHT_IN_DEFAULT
        + (completion_tokens as f64) * WEIGHT_OUT_DEFAULT;
    raw.ceil().max(1.0) as i64
}

/// 端侧 token 估算（接口未返回 usage 时使用）：中文为主，约 1.6 字/token。
pub fn estimate_tokens(chars: usize) -> i64 {
    ((chars as f64) / TOKEN_ESTIMATE_CHARS_PER_TOKEN).ceil() as i64
}

/// 记账入参（一次真实 LLM 请求）。
pub struct QuotaUsageInput<'a> {
    /// 本地日期 `YYYY-MM-DD`（决定归属周期）。
    pub date: &'a str,
    /// 场景：chat / proactive / distill / diary。
    pub scene: &'a str,
    /// 命中链段：builtin（记账）/ custom（不落库）。
    pub tier: &'a str,
    pub model: &'a str,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    /// token 来源：`api`（接口 usage）/ `estimated`（端侧估算）。
    pub token_source: &'a str,
    pub ok: bool,
    /// 失败原因（成功传 None）。
    pub error: Option<&'a str>,
}

/// 是否应记账（**唯一判定出口**）：仅内置额度段。
pub fn should_charge(tier: &str) -> bool {
    tier == "builtin"
}

/// 写入一条明细；命中内置段时同步增量更新月度汇总。
///
/// 幂等性：每次调用即一条明细（failover 多次尝试各一条），不做去重。
/// 失败条目 credits 记 0（不扣点）。
pub fn record(conn: &Connection, input: &QuotaUsageInput<'_>) -> Result<(), AppError> {
    // 自定义 BYOK 不落库、不展示（需求红线）：非内置段直接跳过，调用方可无条件调用
    if !should_charge(input.tier) {
        return Ok(());
    }
    let period_key = period_key_of(input.date);
    let total_tokens = input.prompt_tokens + input.completion_tokens;
    // 失败不扣点；成功按计权换算
    let credits =
        if input.ok { credits_of(input.prompt_tokens, input.completion_tokens) } else { 0 };

    conn.execute(
        "INSERT INTO quota_usage \
         (period_key, scene, tier, model, prompt_tokens, completion_tokens, total_tokens, credits, token_source, ok, error) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            period_key,
            input.scene,
            input.tier,
            input.model,
            input.prompt_tokens,
            input.completion_tokens,
            total_tokens,
            credits,
            input.token_source,
            input.ok as i64,
            input.error,
        ],
    )
    .map_err(AppError::from)?;

    // 汇总缓存：与明细同事务增量 upsert（调用方持有事务时随事务提交）
    conn.execute(
        "INSERT INTO quota_usage_monthly (period_key, granted, used, requests) \
         VALUES (?1, ?2, ?3, 1) \
         ON CONFLICT(period_key) DO UPDATE SET \
             used = used + excluded.used, \
             requests = requests + 1, \
             updated_at = CURRENT_TIMESTAMP",
        params![period_key, QUOTA_MONTHLY_CREDITS, credits],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 读月度汇总（无记录 → 按默认额度返回全量剩余）。
pub fn monthly(conn: &Connection, period_key: &str) -> Result<QuotaUsageMonthlyRow, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT period_key, granted, used, requests FROM quota_usage_monthly \
             WHERE period_key = ?1",
        )
        .map_err(AppError::from)?;
    let mut rows = stmt.query(params![period_key]).map_err(AppError::from)?;
    match rows.next().map_err(AppError::from)? {
        Some(row) => Ok(QuotaUsageMonthlyRow {
            period_key: row.get(0)?,
            granted: row.get(1)?,
            used: row.get(2)?,
            requests: row.get(3)?,
        }),
        None => Ok(QuotaUsageMonthlyRow {
            period_key: period_key.to_string(),
            granted: QUOTA_MONTHLY_CREDITS,
            used: 0,
            requests: 0,
        }),
    }
}

/// 读月度明细（倒序；limit 用于限制条数）。
///
/// 存储口径与全应用一致：`created_at` 为 UTC（`CURRENT_TIMESTAMP`），此处经
/// `datetime(created_at, 'localtime')` 转本地时区再返回，避免前端把 UTC 当本地时间展示
/// （UTC+8 用户会看到慢 8 小时）。排序仍按原始 UTC 列，单调平移不影响顺序。
pub fn recent(
    conn: &Connection,
    period_key: &str,
    limit: i64,
) -> Result<Vec<QuotaUsageDetailRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, datetime(created_at, 'localtime') AS created_at, period_key, scene, tier, \
                    model, prompt_tokens, completion_tokens, total_tokens, credits, token_source, ok, error \
             FROM quota_usage WHERE period_key = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![period_key, limit], |row| {
            Ok(QuotaUsageDetailRow {
                id: row.get(0)?,
                created_at: row.get(1)?,
                period_key: row.get(2)?,
                scene: row.get(3)?,
                tier: row.get(4)?,
                model: row.get(5)?,
                prompt_tokens: row.get(6)?,
                completion_tokens: row.get(7)?,
                total_tokens: row.get(8)?,
                credits: row.get(9)?,
                token_source: row.get(10)?,
                ok: row.get::<_, i64>(11)? != 0,
                error: row.get(12)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 清理早于指定周期键的明细与汇总（保留最近 N 个月）。
pub fn prune_before(conn: &Connection, period_key: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM quota_usage WHERE period_key < ?1", params![period_key])
        .map_err(AppError::from)?;
    conn.execute("DELETE FROM quota_usage_monthly WHERE period_key < ?1", params![period_key])
        .map_err(AppError::from)?;
    Ok(())
}

/// 保留最近 `keep_months` 个自然月（含当月），更早的按月清理（周期键字符串序 = 月序）。
pub fn prune_keep_months(conn: &Connection, keep_months: u32) -> Result<(), AppError> {
    use chrono::Datelike;
    let today = chrono::Local::now().date_naive();
    // 保留窗口：当月 + (keep_months-1) 个前月 → 截断点 = 当月往前数 keep_months-1 个月
    let total = today.year() * 12 + today.month() as i32 - 1 - (keep_months as i32 - 1);
    let cutoff = format!("{:04}-{:02}", total.div_euclid(12), total.rem_euclid(12) + 1);
    prune_before(conn, &cutoff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    fn input<'a>(
        date: &'a str,
        scene: &'a str,
        tier: &'a str,
        prompt: i64,
        completion: i64,
        ok: bool,
        error: Option<&'a str>,
    ) -> QuotaUsageInput<'a> {
        QuotaUsageInput {
            date,
            scene,
            tier,
            model: "glm-4-flash",
            prompt_tokens: prompt,
            completion_tokens: completion,
            token_source: "api",
            ok,
            error,
        }
    }

    #[test]
    fn credits_of_weights_input_and_output_separately() {
        // W_in=1.0 / W_out=2.0：100 输入 + 50 输出 → 100 + 100 = 200
        assert_eq!(credits_of(100, 50), 200);
        // 小数向上取整：1 输入 + 1 输出 → 3.0 → 3
        assert_eq!(credits_of(1, 1), 3);
        // 0 用量仍保底 1 点（"用了就算 1 点"）
        assert_eq!(credits_of(0, 0), 1);
        // 失败条目由 record 层记 0，公式本身不掺 ok 语义
        assert_eq!(credits_of(10, 0), 10);
    }

    #[test]
    fn period_key_of_takes_year_month_prefix() {
        assert_eq!(period_key_of("2026-10-25"), "2026-10");
        assert_eq!(period_key_of("bad"), "bad");
    }

    #[test]
    fn estimate_tokens_rounds_up_by_chars_per_token() {
        assert_eq!(estimate_tokens(16), 10); // 16/1.6 = 10
        assert_eq!(estimate_tokens(17), 11); // 10.625 → 向上取整
        assert_eq!(estimate_tokens(0), 0);
    }

    #[test]
    fn record_builtin_charges_and_upserts_monthly() {
        let (_dir, conn) = fresh_db();
        record(&conn, &input("2026-10-25", "chat", "builtin", 100, 50, true, None))
            .expect("记录内置段");
        let m = monthly(&conn, "2026-10").expect("读汇总");
        assert_eq!(m.granted, QUOTA_MONTHLY_CREDITS);
        assert_eq!(m.used, credits_of(100, 50));
        assert_eq!(m.requests, 1);

        // 累计：第二条相加
        record(&conn, &input("2026-10-26", "chat", "builtin", 10, 0, true, None))
            .expect("记录第二条");
        let m = monthly(&conn, "2026-10").expect("读汇总");
        assert_eq!(m.used, credits_of(100, 50) + credits_of(10, 0));
        assert_eq!(m.requests, 2);
    }

    #[test]
    fn record_failure_keeps_detail_but_zero_credits() {
        let (_dir, conn) = fresh_db();
        record(&conn, &input("2026-10-25", "chat", "builtin", 500, 0, false, Some("timeout")))
            .expect("记录失败条目");
        let m = monthly(&conn, "2026-10").expect("读汇总");
        assert_eq!(m.used, 0, "失败不扣点");
        assert_eq!(m.requests, 1, "但计入请求次数");
        let rows = recent(&conn, "2026-10", 10).expect("读明细");
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].ok);
        assert_eq!(rows[0].credits, 0);
        assert_eq!(rows[0].error.as_deref(), Some("timeout"));
    }

    #[test]
    fn record_custom_never_persists() {
        let (_dir, conn) = fresh_db();
        record(&conn, &input("2026-10-25", "chat", "custom", 1000, 200, true, None))
            .expect("记录自定义段");
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM quota_usage", [], |r| r.get(0)).expect("查明细");
        assert_eq!(count, 0, "自定义 BYOK 不落库");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quota_usage_monthly", [], |r| r.get(0))
            .expect("查汇总");
        assert_eq!(count, 0, "自定义段不进月度汇总");
    }

    #[test]
    fn monthly_absent_period_returns_default_grant() {
        let (_dir, conn) = fresh_db();
        let m = monthly(&conn, "2026-11").expect("读空月");
        assert_eq!(m.period_key, "2026-11");
        assert_eq!(m.granted, QUOTA_MONTHLY_CREDITS);
        assert_eq!(m.used, 0);
        assert_eq!(m.requests, 0);
    }

    #[test]
    fn recent_orders_desc_and_respects_limit() {
        let (_dir, conn) = fresh_db();
        for day in 1..=3 {
            record(&conn, &input("2026-10-25", "chat", "builtin", day, 0, true, None))
                .expect("记录");
        }
        let rows = recent(&conn, "2026-10", 2).expect("读明细");
        assert_eq!(rows.len(), 2);
        assert!(rows[0].prompt_tokens >= rows[1].prompt_tokens, "倒序");
    }

    #[test]
    fn prune_before_removes_only_older_periods() {
        let (_dir, conn) = fresh_db();
        record(&conn, &input("2025-09-15", "chat", "builtin", 10, 0, true, None)).expect("旧月");
        record(&conn, &input("2026-10-25", "chat", "builtin", 10, 0, true, None)).expect("当月");
        prune_before(&conn, "2026-01").expect("清理");
        let rows = recent(&conn, "2026-10", 10).expect("读明细");
        assert_eq!(rows.len(), 1, "旧月被清理");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quota_usage_monthly", [], |r| r.get(0))
            .expect("查汇总");
        assert_eq!(count, 1, "旧月汇总同步清理");
    }

    #[test]
    fn recent_returns_local_time_not_raw_utc() {
        let (_dir, conn) = fresh_db();
        record(&conn, &input("2026-10-25", "chat", "builtin", 1, 0, true, None)).expect("记账");
        // 裸库存储为 UTC（与全应用约定一致）
        let raw: String =
            conn.query_row("SELECT created_at FROM quota_usage", [], |r| r.get(0)).expect("读裸值");
        // recent 必经 localtime 转换，结果应与「裸值 + localtime」一致
        let converted: String = conn
            .query_row("SELECT datetime(created_at, 'localtime') FROM quota_usage", [], |r| {
                r.get(0)
            })
            .expect("读转换值");
        let rows = recent(&conn, "2026-10", 10).expect("读明细");
        assert_eq!(rows[0].created_at, converted, "recent 应返回本地时间而非 UTC 原文");
        // UTC+8 环境下本地值应领先裸值 8 小时；CI(UTC) 下两者相等也属正确，仅校验格式合法
        assert!(chrono::NaiveDateTime::parse_from_str(&raw, "%Y-%m-%d %H:%M:%S").is_ok());
    }
}

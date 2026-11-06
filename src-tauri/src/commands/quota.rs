//! quota 命令域（薄层：转发仓储；内置体验额度查询）。
//!
//! 口径（方案见 `docs/billing/内置体验额度方案.md`）：
//! - 周期键为**本地自然月** YYYY-MM（与记账侧 chrono 同口径）；
//! - 仅返回内置额度数据——自定义 BYOK 本就不落库；
//! - 界面默认读汇总；明细按需展开（审计/对账）。

use rusqlite::Connection;
use tauri::State;

use crate::db::models::{QuotaPlatformUsageDto, QuotaSummaryDto, QuotaUsageRecordDto};
use crate::db::repositories::quota_usage;
use crate::error::AppError;
use crate::state::AppState;

/// 当前本地周期键（记账与查询共用同一 chrono 口径）。
fn current_period() -> String {
    chrono::Local::now().format("%Y-%m").to_string()
}

fn db_lock_err(
    e: std::sync::PoisonError<std::sync::MutexGuard<'_, rusqlite::Connection>>,
) -> AppError {
    AppError::Db(format!("获取数据库连接失败: {e}"))
}

/// 月度汇总（剩余比例由 granted/used 现算，进度条直接可用）。
#[tauri::command]
pub fn quota_summary(state: State<'_, AppState>) -> Result<QuotaSummaryDto, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    quota_summary_inner(&conn, &current_period())
}

pub fn quota_summary_inner(
    conn: &Connection,
    period_key: &str,
) -> Result<QuotaSummaryDto, AppError> {
    let m = quota_usage::monthly(conn, period_key)?;
    let remaining_ratio =
        if m.granted > 0 { (m.granted - m.used).max(0) as f64 / m.granted as f64 } else { 0.0 };
    let platforms = quota_usage::platform_breakdown(conn, period_key)?
        .into_iter()
        .map(|r| QuotaPlatformUsageDto { platform: r.platform, used: r.used, requests: r.requests })
        .collect();
    Ok(QuotaSummaryDto {
        period_key: m.period_key,
        granted: m.granted,
        used: m.used,
        remaining_ratio,
        requests: m.requests,
        platforms,
    })
}

/// 月度明细（倒序；limit 缺省 200，审计/对账用）。
#[tauri::command]
pub fn quota_usage_records(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<QuotaUsageRecordDto>, AppError> {
    let conn = state.db.lock().map_err(db_lock_err)?;
    quota_usage_records_inner(&conn, &current_period(), limit.unwrap_or(200))
}

pub fn quota_usage_records_inner(
    conn: &Connection,
    period_key: &str,
    limit: i64,
) -> Result<Vec<QuotaUsageRecordDto>, AppError> {
    Ok(quota_usage::recent(conn, period_key, limit)?
        .into_iter()
        .map(|r| QuotaUsageRecordDto {
            id: r.id,
            created_at: r.created_at,
            period_key: r.period_key,
            scene: r.scene,
            tier: r.tier,
            platform: r.platform,
            model: r.model,
            prompt_tokens: r.prompt_tokens,
            completion_tokens: r.completion_tokens,
            total_tokens: r.total_tokens,
            credits: r.credits,
            ok: r.ok,
            error: r.error,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn quota_summary_inner_computes_remaining_ratio() {
        let (_dir, conn) = fresh_db();
        quota_usage::record(
            &conn,
            &quota_usage::QuotaUsageInput {
                date: "2026-10-25",
                scene: "chat",
                tier: "builtin",
                platform: "智谱",
                model: "glm-4-flash",
                prompt_tokens: 100,
                completion_tokens: 50,
                token_source: "api",
                ok: true,
                error: None,
            },
        )
        .expect("记账");
        let dto = quota_summary_inner(&conn, "2026-10").expect("读汇总");
        assert_eq!(dto.period_key, "2026-10");
        assert_eq!(dto.used, 200);
        assert_eq!(dto.requests, 1);
        assert_eq!(dto.platforms.len(), 1, "汇总应带平台分项");
        assert_eq!(dto.platforms[0].platform, "智谱");
        assert_eq!(dto.platforms[0].used, 200);
        let expected = (crate::config::QUOTA_MONTHLY_CREDITS - 200) as f64
            / crate::config::QUOTA_MONTHLY_CREDITS as f64;
        assert!((dto.remaining_ratio - expected).abs() < 1e-9);
    }

    #[test]
    fn quota_usage_records_inner_maps_dto() {
        let (_dir, conn) = fresh_db();
        quota_usage::record(
            &conn,
            &quota_usage::QuotaUsageInput {
                date: "2026-10-25",
                scene: "distill",
                tier: "builtin",
                platform: "硅基流动",
                model: "b1",
                prompt_tokens: 10,
                completion_tokens: 5,
                token_source: "estimated",
                ok: false,
                error: Some("超时"),
            },
        )
        .expect("记账");
        let rows = quota_usage_records_inner(&conn, "2026-10", 10).expect("读明细");
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].ok);
        assert_eq!(rows[0].credits, 0, "失败不扣点");
        assert_eq!(rows[0].error.as_deref(), Some("超时"));
        assert_eq!(rows[0].scene, "distill");
        assert_eq!(rows[0].platform, "硅基流动", "明细应带平台名");
    }
}

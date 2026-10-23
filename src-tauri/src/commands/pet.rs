//! pet 命令域（薄层）：桌宠资料读写；**好感度结算/流水查询属 C5（`affection` 域）**。
//!
//! 纪律（commands/ 红线）：本文件只做参数校验 + 转发领域模块；不写业务规则、
//! 不直接拼 SQL、不 emit——emit 在 `affection` 域完成。

use serde::Deserialize;
use tauri::{AppHandle, State};

use rusqlite::Connection;

use crate::affection::rules::AffectionSignal;
use crate::db::models::{AffectionLogDto, PetStateDto, UpdatePetPayload};
use crate::db::repositories::pet_states;
use crate::error::AppError;
use crate::state::AppState;

/// 读取桌宠状态（含好感度只读字段）。
#[tauri::command]
pub fn pet_get_state(app: AppHandle, state: State<'_, AppState>) -> Result<PetStateDto, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    // 懒结算兜底（跨天未重启场景）：静默更新，不 emit（请求/响应模型下无需推事件）
    if let Err(e) = crate::affection::settle_decay_at_startup(&app, &conn) {
        eprintln!("[affection] 懒结算失败（不影响读取）: {e}");
    }
    let row = pet_states::get(&conn)?;
    Ok(PetStateDto {
        pet_name: row.pet_name,
        relationship_level: row.relationship_level,
        affection_points: row.affection_points,
        last_interaction: row.last_interaction,
    })
}

/// 更新桌宠资料（pet_name / active_renderer / renderer_config）。
///
/// 注意：好感度相关字段不接受修改——加分与衰减结算属 C5（`pet_settle_affection`）。
#[tauri::command]
pub fn pet_update_state(
    payload: UpdatePetPayload,
    state: State<'_, AppState>,
) -> Result<PetStateDto, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    pet_update_inner(&conn, payload)
}

/// 纯逻辑（F1.2 可直测）：资料更新（好感度字段只读）。
fn pet_update_inner(conn: &Connection, payload: UpdatePetPayload) -> Result<PetStateDto, AppError> {
    let config_json = payload.renderer_config.as_ref().map(serde_json::to_string).transpose()?;

    pet_states::update_profile(
        conn,
        payload.pet_name.as_deref(),
        payload.active_renderer.as_deref(),
        config_json.as_deref(),
    )?;

    let row = pet_states::get(conn)?;
    Ok(PetStateDto {
        pet_name: row.pet_name,
        relationship_level: row.relationship_level,
        affection_points: row.affection_points,
        last_interaction: row.last_interaction,
    })
}

/// 好感度结算信号（C5 入口）。
///
/// 前端传入信号名：`check_in`（每日首次互动，PetWindow mount 触发）/
/// `pet_stroke`（抚摸，窗口 A 双击/长按）→ 经 `affection::settle_signal`
/// 事务结算并 emit `affection:changed`。`deep_chat`/`resonance` 由 C4 内部调用，
/// 前端不直接发。
#[tauri::command]
pub fn pet_settle_affection(
    signal: AffectionSettleInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::affection::AffectionChangedDto, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    crate::affection::settle_signal(&conn, &app, signal.into_signal())
}

/// 好感度流水查询（D2 看板；limit 分页，默认 50）。
#[tauri::command]
pub fn pet_list_affection_logs(
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<AffectionLogDto>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    pet_list_affection_logs_inner(&conn, limit)
}

/// 纯逻辑（F1.2 可直测）：limit 归一化 + 流水倒序查询。
fn pet_list_affection_logs_inner(
    conn: &Connection,
    limit: Option<i64>,
) -> Result<Vec<AffectionLogDto>, AppError> {
    let limit = limit.unwrap_or(50).clamp(1, 500);
    crate::db::repositories::affection_logs::recent(conn, limit).map(|rows| {
        rows.into_iter()
            .map(|r| AffectionLogDto {
                id: r.id,
                change_value: r.change_value,
                reason: r.reason,
                created_at: r.created_at,
            })
            .collect()
    })
}

/// 结算信号入参（camelCase；与前端 `AffectionSettleInput` 对齐）。
///
/// `resonance`/`taboo` 携带数值——前端不生成，仅供内部/未来扩展。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum AffectionSettleInput {
    CheckIn,
    PetStroke,
    DeepChat,
    Resonance { strength: u8 },
    Taboo { value: i8 },
}

impl AffectionSettleInput {
    /// 转为领域信号枚举（规则引擎消费）。
    fn into_signal(self) -> AffectionSignal {
        match self {
            AffectionSettleInput::CheckIn => AffectionSignal::CheckIn,
            AffectionSettleInput::PetStroke => AffectionSignal::PetStroke,
            AffectionSettleInput::DeepChat => AffectionSignal::DeepChat,
            AffectionSettleInput::Resonance { strength } => AffectionSignal::Resonance(strength),
            AffectionSettleInput::Taboo { value } => AffectionSignal::Taboo(value),
        }
    }
}

// F1.2：命令 inner 直调用（`inner(&conn)` 绕过 Tauri runtime）——验证资料写入与流水读取的 DB 交互。
// 豁免：`pet_get_state`/`pet_settle_affection` 需 `AppHandle` + emit，由 affection 域集成测试覆盖（见覆盖矩阵豁免登记）。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::{affection_logs, pet_states};
    use crate::test_util::fresh_db;

    #[test]
    fn pet_update_state_persists_pet_name_and_keeps_affection() {
        let (_dir, conn) = fresh_db();
        let before = pet_states::get(&conn).unwrap().affection_points;

        let out = pet_update_inner(
            &conn,
            UpdatePetPayload {
                pet_name: Some("团子".into()),
                active_renderer: None,
                renderer_config: None,
            },
        )
        .expect("更新资料");

        assert_eq!(out.pet_name, "团子", "pet_name 应落库回显");
        // 红线：好感度字段只读——update 不得触碰分数
        assert_eq!(out.affection_points, before, "update 不应改好感度");
    }

    #[test]
    fn pet_list_affection_logs_reflects_seed_in_desc_order() {
        let (_dir, conn) = fresh_db();
        affection_logs::insert(&conn, 10, "check_in").unwrap();
        affection_logs::insert(&conn, -20, "taboo_violation").unwrap();
        let logs = pet_list_affection_logs_inner(&conn, Some(50)).expect("查询流水");
        assert_eq!(logs.len(), 2);
        // 倒序（id DESC）：最新一条 taboo 在前
        assert_eq!(logs[0].reason, "taboo_violation");
        assert_eq!(logs[0].change_value, -20);
    }

    #[test]
    fn pet_list_affection_logs_limit_clamped() {
        let (_dir, conn) = fresh_db();
        for _ in 0..3 {
            affection_logs::insert(&conn, 1, "check_in").unwrap();
        }
        // limit=0 → clamp 到 1（默认下限 1）
        let logs = pet_list_affection_logs_inner(&conn, Some(0)).expect("查询");
        assert_eq!(logs.len(), 1, "limit 应被 clamp 到 ≥1");
    }
}

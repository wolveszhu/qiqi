//! pet 命令域（薄层）：桌宠资料读写；**好感度结算/流水查询属 C5（`affection` 域）**。
//!
//! 纪律（commands/ 红线）：本文件只做参数校验 + 转发领域模块；不写业务规则、
//! 不直接拼 SQL、不 emit——emit 在 `affection` 域完成。

use serde::Deserialize;
use tauri::{AppHandle, State};

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

    let config_json = payload.renderer_config.as_ref().map(serde_json::to_string).transpose()?;

    pet_states::update_profile(
        &conn,
        payload.pet_name.as_deref(),
        payload.active_renderer.as_deref(),
        config_json.as_deref(),
    )?;

    let row = pet_states::get(&conn)?;
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
    let limit = limit.unwrap_or(50).clamp(1, 500);
    crate::db::repositories::affection_logs::recent(&conn, limit).map(|rows| {
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

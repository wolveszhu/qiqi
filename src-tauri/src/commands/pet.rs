//! pet 命令域（薄层）：桌宠资料读写；**好感度只读透出，结算逻辑属 C5**。

use tauri::State;

use crate::db::models::{PetStateDto, UpdatePetPayload};
use crate::db::repositories::pet_states;
use crate::error::AppError;
use crate::state::AppState;

/// 读取桌宠状态（含好感度只读字段）。
#[tauri::command]
pub fn pet_get_state(state: State<'_, AppState>) -> Result<PetStateDto, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
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

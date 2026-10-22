//! pet_states 表访问（单用户单角色，id 固定 1）。

use rusqlite::{Connection, params};

use crate::db::models::PetStatesRow;
use crate::error::AppError;

/// 读取 id=1 行；缺失则插入默认行后返回（种子保证存在，此为兜底）。
pub fn get(conn: &Connection) -> Result<PetStatesRow, AppError> {
    match query_row(conn)? {
        Some(row) => Ok(row),
        None => {
            conn.execute("INSERT INTO pet_states (id) VALUES (1)", []).map_err(AppError::from)?;
            query_row(conn)?.ok_or_else(|| AppError::Db("pet_states 初始化失败".into()))
        },
    }
}

fn query_row(conn: &Connection) -> Result<Option<PetStatesRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, pet_name, relationship_level, affection_points,
                    last_decay_at, active_renderer, renderer_config, last_interaction
             FROM pet_states WHERE id = 1",
        )
        .map_err(AppError::from)?;
    let mut rows = stmt.query([]).map_err(AppError::from)?;
    match rows.next().map_err(AppError::from)? {
        Some(row) => Ok(Some(PetStatesRow {
            id: row.get(0)?,
            pet_name: row.get(1)?,
            relationship_level: row.get(2)?,
            affection_points: row.get(3)?,
            last_decay_at: row.get(4)?,
            active_renderer: row.get(5)?,
            renderer_config: row.get(6)?,
            last_interaction: row.get(7)?,
        })),
        None => Ok(None),
    }
}

/// 更新桌宠资料字段（仅传入的字段参与 SET，参数化拼接列名白名单）。
pub fn update_profile(
    conn: &Connection,
    pet_name: Option<&str>,
    active_renderer: Option<&str>,
    renderer_config: Option<&str>,
) -> Result<(), AppError> {
    if let Some(name) = pet_name {
        conn.execute("UPDATE pet_states SET pet_name = ?1 WHERE id = 1", params![name])
            .map_err(AppError::from)?;
    }
    if let Some(renderer) = active_renderer {
        conn.execute("UPDATE pet_states SET active_renderer = ?1 WHERE id = 1", params![renderer])
            .map_err(AppError::from)?;
    }
    if let Some(config) = renderer_config {
        conn.execute("UPDATE pet_states SET renderer_config = ?1 WHERE id = 1", params![config])
            .map_err(AppError::from)?;
    }
    Ok(())
}

/// 仅更新分数与等级（C5 结算落库；不触碰 last_interaction/last_decay_at）。
pub fn set_affection(conn: &Connection, points: i64, level: i64) -> Result<(), AppError> {
    conn.execute(
        "UPDATE pet_states SET affection_points = ?1, relationship_level = ?2 WHERE id = 1",
        params![points, level],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 刷新最后互动日期（互动即推后衰减起点；参数为本地日期串 `YYYY-MM-DD`）。
pub fn touch_interaction(conn: &Connection, date: &str) -> Result<(), AppError> {
    conn.execute("UPDATE pet_states SET last_interaction = ?1 WHERE id = 1", params![date])
        .map_err(AppError::from)?;
    Ok(())
}

/// 刷新最后结算日期（衰减结算后写回；参数为本地日期串 `YYYY-MM-DD`）。
pub fn touch_decay(conn: &Connection, date: &str) -> Result<(), AppError> {
    conn.execute("UPDATE pet_states SET last_decay_at = ?1 WHERE id = 1", params![date])
        .map_err(AppError::from)?;
    Ok(())
}

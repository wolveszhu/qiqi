//! pet 命令域（薄层）：桌宠资料读写；**好感度结算/流水查询属 C5（`affection` 域）**。
//!
//! 纪律（commands/ 红线）：本文件只做参数校验 + 转发领域模块；不写业务规则、
//! 不直接拼 SQL。好感度结算的 emit 在 `affection` 域完成；
//! 本域仅 `pet_update_state` 在形象 id 变化时 emit `pet:renderer:changed`（窗口 A 重建渲染器）。

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

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
    // 懒结算兜底（跨天未重启场景）：静默更新，不 emit（请求/响应模型下无需推事件）。
    // 休眠期跳过：边缘吸附休眠期间衰减暂停（用户决策），防止开设置窗顺手推进结算。
    if !state.is_sleep_mode()
        && let Err(e) = crate::affection::settle_decay_at_startup(&app, &conn)
    {
        eprintln!("[affection] 懒结算失败（不影响读取）: {e}");
    }
    let row = pet_states::get(&conn)?;
    Ok(PetStateDto {
        pet_name: row.pet_name,
        relationship_level: row.relationship_level,
        affection_points: row.affection_points,
        last_interaction: row.last_interaction,
        active_renderer: row.active_renderer,
    })
}

/// 边缘吸附休眠的衰减锚点处理（D4.4；用户决策：休眠期间衰减暂停，醒来再续）。
///
/// - `enter`：先正常结算一次（把入睡前累计的衰减结清），锚点落到今天；
/// - `exit`：仅前移锚点到今天（**不掉分**）——休眠天数被"赦免"，醒来重新起算。
///
/// 为什么需要它：衰减是懒结算（按 `last_decay_at` 跨度计算），若不处理锚点，
/// 一觉睡几天会在唤醒时一次性补扣整个休眠期的衰减。
#[tauri::command]
pub fn pet_sleep_decay(action: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    sleep_decay_inner(&conn, &action)
}

/// inner（薄壳 → inner，便于 `#[test]` 直调；action 非法返回 Config 错误）。
fn sleep_decay_inner(conn: &Connection, action: &str) -> Result<(), AppError> {
    match action {
        "enter" => {
            // 结算到睡前（内部写回 last_decay_at = 今天）
            crate::affection::settle_decay_core(conn)?;
            Ok(())
        },
        "exit" => {
            // 仅前移锚点：赦免休眠天数（不起新结算）
            let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
            pet_states::touch_decay(conn, &today)
        },
        other => Err(AppError::Config(format!("未知休眠衰减动作: {other}"))),
    }
}

/// 更新桌宠资料（pet_name / active_renderer / renderer_config）。
///
/// 注意：好感度相关字段不接受修改——加分与衰减结算属 C5（`pet_settle_affection`）。
///
/// 形象即时生效：仅当 `active_renderer` **确实变化**时才 emit `pet:renderer:changed`
/// （双窗口独立 webview、store 不共享，窗口 A 靠该事件当场重建渲染器；
/// 不改形象时不多读一次库，避免干扰 petName 的 300ms 防抖保存）。
#[tauri::command]
pub fn pet_update_state(
    payload: UpdatePetPayload,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PetStateDto, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    let prev_renderer = if payload.active_renderer.is_some() {
        Some(pet_states::get(&conn)?.active_renderer)
    } else {
        None
    };
    let dto = pet_update_inner(&conn, payload)?; // 纯逻辑（无 AppHandle），可直测
    if let Some(prev) = prev_renderer
        && dto.active_renderer != prev
    {
        let _ = app.emit("pet:renderer:changed", &dto.active_renderer);
    }
    Ok(dto)
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
        active_renderer: row.active_renderer,
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
    fn sleep_decay_enter_settles_and_exit_forgives_sleep_days() {
        let (_dir, conn) = fresh_db();
        // 构造"欠衰减"状态：last_interaction 5 天前、last_decay_at 5 天前、满分档
        let five_days_ago = (chrono::Local::now().date_naive() - chrono::Duration::days(5))
            .format("%Y-%m-%d")
            .to_string();
        conn.execute(
            "UPDATE pet_states SET affection_points = 500, last_interaction = ?1, last_decay_at = ?1 WHERE id = 1",
            rusqlite::params![five_days_ago],
        )
        .expect("构造欠衰减");

        // enter：先结清睡前累计（5 天 × -5，止于档下限前）
        sleep_decay_inner(&conn, "enter").expect("enter");
        let row = pet_states::get(&conn).expect("读取");
        assert!(row.affection_points < 500, "enter 应结清入睡前累计衰减");
        assert_eq!(
            row.last_decay_at.as_deref(),
            Some(chrono::Local::now().date_naive().format("%Y-%m-%d").to_string().as_str()),
            "enter 后锚点落到今天"
        );

        // exit：仅前移锚点，不掉分（赦免休眠天数）。
        // 同日 enter/exit 锚点同为今天属正确行为；此处把锚点拨回"入睡日"模拟跨天休眠，
        // 验证 exit 把它前移到今天（否则唤醒时会一次性补扣整个休眠期）。
        let sleep_day = (chrono::Local::now().date_naive() - chrono::Duration::days(4))
            .format("%Y-%m-%d")
            .to_string();
        pet_states::touch_decay(&conn, &sleep_day).expect("拨回入睡日锚点");
        let before_exit = pet_states::get(&conn).expect("读取").affection_points;
        sleep_decay_inner(&conn, "exit").expect("exit");
        let row = pet_states::get(&conn).expect("读取");
        assert_eq!(row.affection_points, before_exit, "exit 不掉分");
        assert_eq!(
            row.last_decay_at.as_deref(),
            Some(chrono::Local::now().date_naive().format("%Y-%m-%d").to_string().as_str()),
            "exit 前移锚点到今天（赦免休眠天数）"
        );

        // 非法动作 → Config 错误（薄层校验）
        assert!(sleep_decay_inner(&conn, "bogus").is_err(), "未知动作应报错");
    }

    #[test]
    fn sleep_decay_enter_is_idempotent_same_day() {
        let (_dir, conn) = fresh_db();
        sleep_decay_inner(&conn, "enter").expect("首次 enter");
        let p1 = pet_states::get(&conn).expect("读取").affection_points;
        sleep_decay_inner(&conn, "enter").expect("再次 enter（同日幂等）");
        let p2 = pet_states::get(&conn).expect("读取").affection_points;
        assert_eq!(p1, p2, "同日重复 enter 不重复扣分");
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

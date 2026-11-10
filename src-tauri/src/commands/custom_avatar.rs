//! 自定义形象命令域（薄层）：导入 / 列表 / 读取图片 / 改配置 / 重命名 / 删除。
//!
//! 边界（baseline §G）：图片的**读取、拷贝、落库全在本层经 `std::fs` 完成**，前端只经
//! `tauri-plugin-dialog` 的 `open()` 取路径字符串，绝不接触文件系统。
//!
//! 纪律（commands/ 红线）：本文件只做参数校验 + 转发仓储；emit 仅在删除「当前使用形象」时
//! 回退 `code_cat` 并触发重建——与 `pet_update_state` 的 `pet:renderer:changed` 同源。

use std::path::PathBuf;

use base64::Engine as _;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::models::CustomAvatarMetaDto;
use crate::db::repositories::custom_avatars;
use crate::error::AppError;
use crate::state::AppState;

/// 单张导入图体积上限（10 MiB；PNG 透明桌宠图足够，超界即拒绝，防滥用）。
const MAX_AVATAR_BYTES: u64 = 10 * 1024 * 1024;
/// PNG 文件头魔数（8 字节签名）。
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// 缩放合法区间（过小不可见 / 过大超出窗）。
const SCALE_MIN: f64 = 0.1;
const SCALE_MAX: f64 = 10.0;

/// 导入自定义形象：校验 PNG → 拷入 app_data_dir/custom_avatars/<id>/avatar.png → 落库。
///
/// 入参 `src_path` 来自 `tauri-plugin-dialog` 的 `open()`（仅路径字符串，前端不读文件）。
/// 返回元数据（不含图片字节；渲染时由 `read_custom_avatar` 按需取）。
#[tauri::command]
pub fn import_custom_avatar(
    name: String,
    src_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CustomAvatarMetaDto, AppError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Config("形象名称不能为空".into()));
    }

    // 源文件校验：存在 / 是 PNG / 不超体积。
    let src = PathBuf::from(&src_path);
    if !src.is_file() {
        return Err(AppError::Config(format!("文件不存在: {src_path}")));
    }
    let bytes = std::fs::read(&src).map_err(|e| AppError::Io(format!("读取源文件失败: {e}")))?;
    if bytes.len() as u64 > MAX_AVATAR_BYTES {
        return Err(AppError::Config(format!(
            "图片过大（{} MB），上限 {} MB",
            bytes.len() / (1024 * 1024),
            MAX_AVATAR_BYTES / (1024 * 1024)
        )));
    }
    if bytes.len() < PNG_SIGNATURE.len() || !bytes.starts_with(&PNG_SIGNATURE) {
        return Err(AppError::Config("仅支持 PNG 透明图片（.png）".into()));
    }

    // 目标目录：app_data_dir/custom_avatars/<id>
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("解析 app_data_dir 失败: {e}")))?;
    let id = gen_id()?;
    let dest_dir = data_dir.join("custom_avatars").join(&id);
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| AppError::Io(format!("创建形象目录失败: {e}")))?;
    let dest = dest_dir.join("avatar.png");
    std::fs::write(&dest, &bytes).map_err(|e| AppError::Io(format!("写入形象图片失败: {e}")))?;
    let rel_path = format!("custom_avatars/{id}/avatar.png");

    let now = custom_avatars::now_iso();
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    custom_avatars::insert(&conn, &id, &name, &rel_path, 1.0, 0.5, 0.5, &now)?;

    Ok(CustomAvatarMetaDto {
        id,
        name,
        scale: 1.0,
        anchor_x: 0.5,
        anchor_y: 0.5,
    })
}

/// 列出全部自定义形象（不含图片字节；渲染按需 `read_custom_avatar`）。
#[tauri::command]
pub fn list_custom_avatars(
    state: State<'_, AppState>,
) -> Result<Vec<CustomAvatarMetaDto>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    let rows = custom_avatars::list_all(&conn)?;
    Ok(rows
        .into_iter()
        .map(|r| CustomAvatarMetaDto {
            id: r.id,
            name: r.name,
            scale: r.scale,
            anchor_x: r.anchor_x,
            anchor_y: r.anchor_y,
        })
        .collect())
}

/// 读取自定义形象的图片（返回 `data:image/png;base64,...`，前端 `PIXI Assets.load` 建纹理）。
///
/// 仅返回图片字节，不落前端存储；`customAvatarRegistry` 在内存中缓存避免重复 IPC。
#[tauri::command]
pub fn read_custom_avatar(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("解析 app_data_dir 失败: {e}")))?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    let row = custom_avatars::get(&conn, &id)?
        .ok_or_else(|| AppError::Config(format!("自定义形象不存在: {id}")))?;
    let path = data_dir.join(&row.rel_path);
    let bytes = std::fs::read(&path).map_err(|e| AppError::Io(format!("读取形象图片失败: {e}")))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

/// 更新缩放与锚点（微调 UI 实时写回）。
#[tauri::command]
pub fn update_custom_avatar_config(
    id: String,
    scale: f64,
    anchor_x: f64,
    anchor_y: f64,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let scale = scale.clamp(SCALE_MIN, SCALE_MAX);
    let anchor_x = anchor_x.clamp(0.0, 1.0);
    let anchor_y = anchor_y.clamp(0.0, 1.0);
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    if custom_avatars::get(&conn, &id)?.is_none() {
        return Err(AppError::Config(format!("自定义形象不存在: {id}")));
    }
    custom_avatars::update_config(&conn, &id, scale, anchor_x, anchor_y)?;
    Ok(())
}

/// 重命名自定义形象。
#[tauri::command]
pub fn rename_custom_avatar(
    id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Config("形象名称不能为空".into()));
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    if custom_avatars::get(&conn, &id)?.is_none() {
        return Err(AppError::Config(format!("自定义形象不存在: {id}")));
    }
    custom_avatars::rename(&conn, &id, &name)?;
    Ok(())
}

/// 删除自定义形象：删库行 + 删图片文件；若正被使用则回退 `code_cat` 并 emit 重建。
#[tauri::command]
pub fn delete_custom_avatar(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("解析 app_data_dir 失败: {e}")))?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Db(format!("获取数据库连接失败: {e}")))?;
    let was_active = custom_avatars::is_active(&conn, &id)?;
    custom_avatars::delete(&conn, &id)?;
    // 删图片目录（best-effort，失败仅 log，不阻断删除）。
    let dir = data_dir.join("custom_avatars").join(&id);
    if dir.exists() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    if was_active {
        // 回退到内置猫并通知窗口 A 重建，避免孤儿引用。
        use crate::db::repositories::pet_states;
        pet_states::update_profile(&conn, None, Some("code_cat"), None)?;
        let _ = app.emit("pet:renderer:changed", &"code_cat".to_string());
    }
    Ok(())
}

/// 生成唯一 id（`ca_<秒级时间戳>_<8hex>`，量级为个位数形象，碰撞可忽略）。
fn gen_id() -> Result<String, AppError> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut buf = [0u8; 4];
    getrandom::getrandom(&mut buf).map_err(|e| AppError::Io(format!("生成形象 id 失败: {e}")))?;
    let suffix: String = buf.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("ca_{secs}_{suffix}"))
}

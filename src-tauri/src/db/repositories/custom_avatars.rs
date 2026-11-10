//! custom_avatars 表访问（用户导入的自定义形象，与内置猫狗并列）。
//!
//! 纪律（规范 §4.1）：只写参数化 SQL、不做业务决策、不 emit 事件、不碰文件系统；
//! 图片的拷贝/读取由命令层经 `std::fs` 完成（baseline §G：前端绝不碰文件系统）。
//! 行结构体就近定义在此（含明文 `rel_path`，但属本地相对路径，非密钥，无扩散风险）。

use rusqlite::{Connection, params};

use crate::error::AppError;

/// 自定义形象行（内部使用）。
#[derive(Debug, Clone)]
pub struct CustomAvatarRow {
    pub id: String,
    pub name: String,
    /// 相对 app_data_dir 的路径（如 `custom_avatars/<id>/avatar.png`）。
    pub rel_path: String,
    /// 整体缩放（默认 1）。
    pub scale: f64,
    /// 圆盘中心 X（归一化 0..1，默认 0.5）。
    pub anchor_x: f64,
    /// 圆盘中心 Y（归一化 0..1，默认 0.5）。
    pub anchor_y: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// 列出全部自定义形象（按 created_at ASC；管理列表用）。
pub fn list_all(conn: &Connection) -> Result<Vec<CustomAvatarRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, rel_path, scale, anchor_x, anchor_y, created_at, updated_at
             FROM custom_avatars ORDER BY created_at ASC, id ASC",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CustomAvatarRow {
                id: row.get(0)?,
                name: row.get(1)?,
                rel_path: row.get(2)?,
                scale: row.get(3)?,
                anchor_x: row.get(4)?,
                anchor_y: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 读取单条（不存在返回 None）。
pub fn get(conn: &Connection, id: &str) -> Result<Option<CustomAvatarRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, rel_path, scale, anchor_x, anchor_y, created_at, updated_at
             FROM custom_avatars WHERE id = ?1",
        )
        .map_err(AppError::from)?;
    let mut rows = stmt.query(params![id]).map_err(AppError::from)?;
    match rows.next().map_err(AppError::from)? {
        Some(row) => Ok(Some(CustomAvatarRow {
            id: row.get(0)?,
            name: row.get(1)?,
            rel_path: row.get(2)?,
            scale: row.get(3)?,
            anchor_x: row.get(4)?,
            anchor_y: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })),
        None => Ok(None),
    }
}

/// 新增自定义形象（路径/几何默认值由命令层传入，本层只负责持久化）。
#[allow(clippy::too_many_arguments)]
pub fn insert(
    conn: &Connection,
    id: &str,
    name: &str,
    rel_path: &str,
    scale: f64,
    anchor_x: f64,
    anchor_y: f64,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO custom_avatars (id, name, rel_path, scale, anchor_x, anchor_y, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![id, name, rel_path, scale, anchor_x, anchor_y, now],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 更新缩放与锚点（微调 UI 实时写回；参数均已在命令层 clamp）。
pub fn update_config(
    conn: &Connection,
    id: &str,
    scale: f64,
    anchor_x: f64,
    anchor_y: f64,
) -> Result<(), AppError> {
    let now = now_iso();
    conn.execute(
        "UPDATE custom_avatars SET scale = ?1, anchor_x = ?2, anchor_y = ?3, updated_at = ?4
         WHERE id = ?5",
        params![scale, anchor_x, anchor_y, now, id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 重命名（name 已在命令层校验非空）。
pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<(), AppError> {
    let now = now_iso();
    conn.execute(
        "UPDATE custom_avatars SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![name, now, id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 删除自定义形象（仅清库行；图片文件由命令层负责清理）。
pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM custom_avatars WHERE id = ?1", params![id])
        .map_err(AppError::from)?;
    Ok(())
}

/// 当前 `active_renderer` 是否指向该自定义形象（`custom:<id>`）。
pub fn is_active(conn: &Connection, id: &str) -> Result<bool, AppError> {
    let active: String = conn
        .query_row(
            "SELECT active_renderer FROM pet_states WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;
    Ok(active == format!("custom:{id}"))
}

/// 当前时间戳（本地时区 ISO，与 pet_states 等一致）。
pub fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn insert_and_list_roundtrip() {
        let (_dir, conn) = fresh_db();
        insert(
            &conn, "ca_1", "团子", "custom_avatars/ca_1/avatar.png", 1.0, 0.5, 0.5, &now_iso(),
        )
        .expect("插入");

        let all = list_all(&conn).expect("列表");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "团子");
        assert_eq!(all[0].scale, 1.0);

        let got = get(&conn, "ca_1").expect("读取").expect("存在");
        assert_eq!(got.rel_path, "custom_avatars/ca_1/avatar.png");
        assert!(!is_active(&conn, "ca_1").expect("判活"), "默认非当前");
    }

    #[test]
    fn update_config_changes_scale_and_anchor() {
        let (_dir, conn) = fresh_db();
        insert(
            &conn, "ca_1", "团子", "custom_avatars/ca_1/avatar.png", 1.0, 0.5, 0.5, &now_iso(),
        )
        .expect("插入");
        update_config(&conn, "ca_1", 1.8, 0.3, 0.7).expect("更新配置");
        let got = get(&conn, "ca_1").expect("读取").expect("存在");
        assert_eq!(got.scale, 1.8);
        assert_eq!(got.anchor_x, 0.3);
        assert_eq!(got.anchor_y, 0.7);
    }

    #[test]
    fn rename_updates_name_and_delete_removes_row() {
        let (_dir, conn) = fresh_db();
        insert(
            &conn, "ca_1", "旧名", "custom_avatars/ca_1/avatar.png", 1.0, 0.5, 0.5, &now_iso(),
        )
        .expect("插入");
        rename(&conn, "ca_1", "新名").expect("重命名");
        assert_eq!(get(&conn, "ca_1").expect("读").expect("在").name, "新名");
        delete(&conn, "ca_1").expect("删除");
        assert!(get(&conn, "ca_1").expect("读").is_none());
    }

    #[test]
    fn is_active_detects_custom_id() {
        let (_dir, conn) = fresh_db();
        insert(
            &conn, "ca_9", "X", "custom_avatars/ca_9/avatar.png", 1.0, 0.5, 0.5, &now_iso(),
        )
        .expect("插入");
        conn.execute(
            "UPDATE pet_states SET active_renderer = 'custom:ca_9' WHERE id = 1",
            [],
        )
        .expect("设为当前");
        assert!(is_active(&conn, "ca_9").expect("判活"), "应识别为当前");
        assert!(!is_active(&conn, "ca_1").expect("判活"), "其它 id 不是当前");
    }
}

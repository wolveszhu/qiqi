//! custom_translate_providers 表访问（用户自填多个翻译服务商，优先于内置）。
//!
//! 与 `custom_model_settings` 对称：多 `sort_order`（列表排序 / 自填优先顺序）。
//! 纪律（规范 §4.1）：只写参数化 SQL、不做业务决策、不 emit 事件；
//! `api_key` 明文只在本层与命令层之间流转，**禁止进日志 / 错误 / DTO**。

use rusqlite::{Connection, params};

use crate::error::AppError;

/// 自填服务商行（`api_key` 为明文，仅内部使用）。
#[derive(Debug, Clone)]
pub struct CustomTranslateProviderRow {
    pub id: i64,
    pub name: String,
    /// baidu / deepl / youdao / google。
    pub provider: String,
    /// 应用 ID（百度 / 有道需要）。
    pub app_id: String,
    pub api_key: String,
    /// 1=启用 / 2=停用。
    pub is_can_use: i64,
    pub sort_order: i64,
}

impl CustomTranslateProviderRow {
    /// 是否启用。
    pub fn is_enabled(&self) -> bool {
        self.is_can_use == 1
    }

    /// 凭据是否齐全：Key 必填，百度 / 有道还要求 APPID。
    pub fn has_credentials(&self) -> bool {
        if self.api_key.is_empty() {
            return false;
        }
        !matches!(self.provider.as_str(), "baidu" | "youdao") || !self.app_id.is_empty()
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> Result<CustomTranslateProviderRow, rusqlite::Error> {
    Ok(CustomTranslateProviderRow {
        id: row.get(0)?,
        name: row.get(1)?,
        provider: row.get(2)?,
        app_id: row.get(3)?,
        api_key: row.get(4)?,
        is_can_use: row.get(5)?,
        sort_order: row.get(6)?,
    })
}

/// 全部自填服务商（按 sort_order ASC, id ASC；管理列表用）。
pub fn list_all(conn: &Connection) -> Result<Vec<CustomTranslateProviderRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, provider, app_id, api_key, is_can_use, sort_order \
             FROM custom_translate_providers ORDER BY sort_order ASC, id ASC",
        )
        .map_err(AppError::from)?;
    let rows = stmt.query_map([], map_row).map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 可用于翻译的自填服务商（启用且凭据齐全，按排序）；空 Vec 表示回退内置通道。
pub fn list_usable(conn: &Connection) -> Result<Vec<CustomTranslateProviderRow>, AppError> {
    Ok(list_all(conn)?.into_iter().filter(|r| r.is_enabled() && r.has_credentials()).collect())
}

/// 读取单条（不存在返回 None）。
pub fn get(conn: &Connection, id: i64) -> Result<Option<CustomTranslateProviderRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, provider, app_id, api_key, is_can_use, sort_order \
             FROM custom_translate_providers WHERE id = ?1",
        )
        .map_err(AppError::from)?;
    let mut rows = stmt.query(params![id]).map_err(AppError::from)?;
    match rows.next().map_err(AppError::from)? {
        Some(row) => Ok(Some(map_row(row).map_err(AppError::from)?)),
        None => Ok(None),
    }
}

/// 新增自填服务商；sort_order 自动追加到队尾。
pub fn insert(
    conn: &Connection,
    name: &str,
    provider: &str,
    app_id: &str,
    api_key: &str,
) -> Result<i64, AppError> {
    let next_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM custom_translate_providers",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;
    conn.execute(
        "INSERT INTO custom_translate_providers \
         (name, provider, app_id, api_key, is_can_use, sort_order) \
         VALUES (?1, ?2, ?3, ?4, 1, ?5)",
        params![name, provider, app_id, api_key, next_order],
    )
    .map_err(AppError::from)?;
    Ok(conn.last_insert_rowid())
}

/// 更新自填服务商（不含 api_key：Key 变更走 `update_api_key`，避免误清空）。
pub fn update(
    conn: &Connection,
    id: i64,
    name: &str,
    provider: &str,
    app_id: &str,
    is_can_use: i64,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE custom_translate_providers \
         SET name = ?1, provider = ?2, app_id = ?3, is_can_use = ?4 WHERE id = ?5",
        params![name, provider, app_id, is_can_use, id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 单独更新 Key（`""` 表示显式清空）。
pub fn update_api_key(conn: &Connection, id: i64, api_key: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE custom_translate_providers SET api_key = ?1 WHERE id = ?2",
        params![api_key, id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 删除自填服务商。
pub fn delete(conn: &Connection, id: i64) -> Result<(), AppError> {
    conn.execute("DELETE FROM custom_translate_providers WHERE id = ?1", params![id])
        .map_err(AppError::from)?;
    Ok(())
}

/// 上移 / 下移一位（相邻交换 sort_order）；越界或不存在返回 Ok(false)。
pub fn move_by_one(conn: &Connection, id: i64, up: bool) -> Result<bool, AppError> {
    let rows = list_all(conn)?;
    let Some(pos) = rows.iter().position(|r| r.id == id) else {
        return Ok(false);
    };
    let target = if up {
        pos.checked_sub(1)
    } else {
        let next = pos + 1;
        (next < rows.len()).then_some(next)
    };
    let Some(other_pos) = target else {
        return Ok(false);
    };
    let (Some(current), Some(other)) = (rows.get(pos), rows.get(other_pos)) else {
        return Ok(false);
    };
    let (current_order, current_id) = (current.sort_order, current.id);
    let (other_order, other_id) = (other.sort_order, other.id);
    conn.execute(
        "UPDATE custom_translate_providers SET sort_order = ?1 WHERE id = ?2",
        params![other_order, current_id],
    )
    .map_err(AppError::from)?;
    conn.execute(
        "UPDATE custom_translate_providers SET sort_order = ?1 WHERE id = ?2",
        params![current_order, other_id],
    )
    .map_err(AppError::from)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn insert_appends_sort_order_and_lists_usable() {
        let (_dir, conn) = fresh_db();
        insert(&conn, "我的 DeepL", "deepl", "", "key-1").expect("新增 1");
        insert(&conn, "我的有道", "youdao", "app-1", "key-2").expect("新增 2");
        let all = list_all(&conn).expect("列表");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].sort_order, 0);
        assert_eq!(all[1].sort_order, 1);
        assert_eq!(list_usable(&conn).expect("可用").len(), 2);
    }

    #[test]
    fn usable_filters_disabled_and_missing_credentials() {
        let (_dir, conn) = fresh_db();
        let id = insert(&conn, "空 Key", "deepl", "", "").expect("新增");
        assert!(list_usable(&conn).expect("可用").is_empty(), "无 Key 不可用");
        update(&conn, id, "空 Key", "deepl", "", 2).expect("停用");
        update_api_key(&conn, id, "key-1").expect("补 Key");
        assert!(list_usable(&conn).expect("可用").is_empty(), "停用不可用");
        update(&conn, id, "空 Key", "deepl", "", 1).expect("启用");
        assert_eq!(list_usable(&conn).expect("可用").len(), 1);
    }

    #[test]
    fn baidu_and_youdao_require_app_id() {
        let (_dir, conn) = fresh_db();
        let id = insert(&conn, "我的百度", "baidu", "", "key-1").expect("新增");
        assert!(list_usable(&conn).expect("可用").is_empty(), "百度缺 APPID 不可用");
        update(&conn, id, "我的百度", "baidu", "app-1", 1).expect("补 APPID");
        assert_eq!(list_usable(&conn).expect("可用").len(), 1);
    }

    #[test]
    fn move_by_one_swaps_order_and_respects_bounds() {
        let (_dir, conn) = fresh_db();
        let a = insert(&conn, "A", "deepl", "", "key-a").expect("A");
        let b = insert(&conn, "B", "google", "", "key-b").expect("B");
        assert!(!move_by_one(&conn, a, true).expect("A 上移越界"));
        assert!(move_by_one(&conn, b, true).expect("B 上移"));
        let all = list_all(&conn).expect("列表");
        assert_eq!(all[0].id, b, "B 应排到首位");
        assert_eq!(all[1].id, a);
        assert!(!move_by_one(&conn, a, false).expect("A 下移越界"));
    }

    #[test]
    fn update_keeps_key_and_delete_removes_row() {
        let (_dir, conn) = fresh_db();
        let id = insert(&conn, "A", "deepl", "", "key-a").expect("新增");
        update(&conn, id, "改名", "google", "", 1).expect("更新");
        let row = get(&conn, id).expect("读取").expect("存在");
        assert_eq!(row.name, "改名");
        assert_eq!(row.api_key, "key-a", "update 不得清空 Key");
        delete(&conn, id).expect("删除");
        assert!(get(&conn, id).expect("读取").is_none());
    }
}

//! custom_model_settings 表访问（用户自定义大模型，优先于内置）。
//!
//! 与 `default_model_settings` 对称但多 `sort_order`（前端列表排序）。
//! 纪律（规范 §4.1）：只写参数化 SQL、不做业务决策、不 emit 事件；
//! `api_key` 明文只在本层与命令层之间流转，**禁止进日志/错误/DTO**。

use rusqlite::{Connection, params};

use super::{parse_model_list, serialize_model_list};
use crate::error::AppError;

/// 自定义模型行（`api_key` 为明文，仅内部使用）。
#[derive(Debug, Clone)]
pub struct CustomModelRow {
    pub id: i64,
    pub name: String,
    /// 1=LLM, 2=语音, 3=图像, 4=视频, 5=翻译（与内置表同义；编排只用 1）。
    pub setting_type: i64,
    pub base_url: String,
    /// 模型名数组（库内 JSON 字符串形态）。
    pub models_json: String,
    pub api_key: String,
    /// 1=可用 / 2=停用。
    pub is_can_use: i64,
    pub sort_order: i64,
}

impl CustomModelRow {
    /// 模型名列表（解析失败 → 空 Vec，见 `super::parse_model_list`）。
    pub fn models(&self) -> Vec<String> {
        parse_model_list(&self.models_json)
    }

    /// 是否启用（`is_can_use = 1`）。
    pub fn is_enabled(&self) -> bool {
        self.is_can_use == 1
    }
}

/// 全部自定义模型（按 sort_order ASC, id ASC；管理列表用）。
pub fn list_all(conn: &Connection) -> Result<Vec<CustomModelRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, type, base_url, models, api_key, is_can_use, sort_order
             FROM custom_model_settings ORDER BY sort_order ASC, id ASC",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CustomModelRow {
                id: row.get(0)?,
                name: row.get(1)?,
                setting_type: row.get(2)?,
                base_url: row.get(3)?,
                models_json: row.get(4)?,
                api_key: row.get(5)?,
                is_can_use: row.get(6)?,
                sort_order: row.get(7)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 可用（启用）且类型为 LLM 的自定义模型（编排链数据源；按排序）。
pub fn list_enabled_llm(conn: &Connection) -> Result<Vec<CustomModelRow>, AppError> {
    Ok(list_all(conn)?.into_iter().filter(|r| r.is_enabled() && r.setting_type == 1).collect())
}

/// 读取单条（不存在返回 None）。
pub fn get(conn: &Connection, id: i64) -> Result<Option<CustomModelRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, type, base_url, models, api_key, is_can_use, sort_order
             FROM custom_model_settings WHERE id = ?1",
        )
        .map_err(AppError::from)?;
    let mut rows = stmt.query(params![id]).map_err(AppError::from)?;
    match rows.next().map_err(AppError::from)? {
        Some(row) => Ok(Some(CustomModelRow {
            id: row.get(0)?,
            name: row.get(1)?,
            setting_type: row.get(2)?,
            base_url: row.get(3)?,
            models_json: row.get(4)?,
            api_key: row.get(5)?,
            is_can_use: row.get(6)?,
            sort_order: row.get(7)?,
        })),
        None => Ok(None),
    }
}

/// 新增自定义模型（`api_key` 空串 = 占位；`models` 空数组 → `[]`）。
/// sort_order 自动追加到当前队尾（`max(sort_order)+1`）。
pub fn insert(
    conn: &Connection,
    name: &str,
    base_url: &str,
    models: &[String],
    api_key: &str,
) -> Result<i64, AppError> {
    let models_json = serialize_model_list(models)?;
    let next_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM custom_model_settings",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;
    conn.execute(
        "INSERT INTO custom_model_settings (name, type, base_url, models, api_key, is_can_use, sort_order)
         VALUES (?1, 1, ?2, ?3, ?4, 1, ?5)",
        params![name, base_url, models_json, api_key, next_order],
    )
    .map_err(AppError::from)?;
    Ok(conn.last_insert_rowid())
}

/// 更新自定义模型（不含 api_key：Key 变更走 `update_api_key`，避免误清空）。
pub fn update(
    conn: &Connection,
    id: i64,
    name: &str,
    base_url: &str,
    models: &[String],
    is_can_use: i64,
) -> Result<(), AppError> {
    let models_json = serialize_model_list(models)?;
    conn.execute(
        "UPDATE custom_model_settings SET name = ?1, base_url = ?2, models = ?3, is_can_use = ?4
         WHERE id = ?5",
        params![name, base_url, models_json, is_can_use, id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 更新 API Key（空串 = 清空；前端"留空=不改动"语义由命令层实现）。
pub fn update_api_key(conn: &Connection, id: i64, api_key: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE custom_model_settings SET api_key = ?1 WHERE id = ?2",
        params![api_key, id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 删除自定义模型。
pub fn delete(conn: &Connection, id: i64) -> Result<(), AppError> {
    conn.execute("DELETE FROM custom_model_settings WHERE id = ?1", params![id])
        .map_err(AppError::from)?;
    Ok(())
}

/// 上移/下移一位（与相邻项交换 sort_order；已在顶端/底部则空操作）。
///
/// 返回是否发生了移动（供命令层回执/测试断言）。
pub fn move_by_one(conn: &Connection, id: i64, up: bool) -> Result<bool, AppError> {
    let all = list_all(conn)?;
    let Some(idx) = all.iter().position(|r| r.id == id) else {
        return Ok(false);
    };
    let neighbor = if up { idx.checked_sub(1) } else { idx.checked_add(1) };
    let Some(nidx) = neighbor.filter(|n| *n < all.len()) else {
        return Ok(false); // 顶/底：无移动
    };
    let (Some(a), Some(b)) = (all.get(idx), all.get(nidx)) else {
        return Ok(false); // 下标理论上有效（position/filter 已保证），防御性兜底
    };
    conn.execute(
        "UPDATE custom_model_settings SET sort_order = ?1 WHERE id = ?2",
        params![b.sort_order, a.id],
    )
    .map_err(AppError::from)?;
    conn.execute(
        "UPDATE custom_model_settings SET sort_order = ?1 WHERE id = ?2",
        params![a.sort_order, b.id],
    )
    .map_err(AppError::from)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    fn seed(conn: &Connection, name: &str, models: &[&str]) -> i64 {
        let models: Vec<String> = models.iter().map(|s| (*s).to_string()).collect();
        insert(conn, name, "https://api.example.com/v1", &models, "sk-test1234").expect("插入")
    }

    #[test]
    fn insert_appends_sort_order_and_parses_models() {
        let (_dir, conn) = fresh_db();
        let a = seed(&conn, "甲", &["m-a"]);
        let b = seed(&conn, "乙", &["m-b1", "m-b2"]);

        let all = list_all(&conn).expect("列表");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, a, "先插入的排前面");
        assert!(all[0].sort_order < all[1].sort_order, "sort_order 递增");
        assert_eq!(all[1].id, b);
        assert_eq!(all[1].models(), vec!["m-b1", "m-b2"], "models JSON 应解析为数组");
    }

    #[test]
    fn list_enabled_llm_filters_disabled_and_non_llm() {
        let (_dir, conn) = fresh_db();
        let keep = seed(&conn, "启用", &["m"]);
        let off = seed(&conn, "停用", &["m"]);
        update(&conn, off, "停用", "https://api.example.com/v1", &["m".into()], 2).expect("停用");
        // 手工插入一条非 LLM 类型（语音），验证过滤
        conn.execute(
            "INSERT INTO custom_model_settings (name, type, base_url, models, api_key, is_can_use, sort_order)
             VALUES ('语音', 2, 'https://x/v1', '[]', 'k', 1, 99)",
            [],
        )
        .expect("插入语音");

        let enabled = list_enabled_llm(&conn).expect("可用列表");
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, keep);
    }

    #[test]
    fn move_by_one_swaps_neighbors_and_clamps_at_edges() {
        let (_dir, conn) = fresh_db();
        let a = seed(&conn, "甲", &["m"]);
        let b = seed(&conn, "乙", &["m"]);
        let c = seed(&conn, "丙", &["m"]);

        assert!(!move_by_one(&conn, a, true).expect("顶部上移"), "顶部上移无效");
        assert!(move_by_one(&conn, b, true).expect("上移"), "中间可上移");
        let order: Vec<i64> = list_all(&conn).expect("列表").into_iter().map(|r| r.id).collect();
        assert_eq!(order, vec![b, a, c], "上移后乙到头");

        assert!(!move_by_one(&conn, c, false).expect("底下移"), "底下移无效");
        assert!(move_by_one(&conn, b, false).expect("下移"));
        let order: Vec<i64> = list_all(&conn).expect("列表").into_iter().map(|r| r.id).collect();
        assert_eq!(order, vec![a, b, c], "下移后回到原序");
    }

    #[test]
    fn update_keeps_api_key_and_delete_removes_row() {
        let (_dir, conn) = fresh_db();
        let id = seed(&conn, "原名", &["old"]);
        update(&conn, id, "新名", "https://new/v1", &["new".into()], 1).expect("更新");

        let row = get(&conn, id).expect("读取").expect("存在");
        assert_eq!(row.name, "新名");
        assert_eq!(row.base_url, "https://new/v1");
        assert_eq!(row.api_key, "sk-test1234", "update 不动 api_key（防误清空）");

        delete(&conn, id).expect("删除");
        assert!(get(&conn, id).expect("读取").is_none());
    }

    #[test]
    fn bad_models_json_yields_empty_list_not_error() {
        let (_dir, conn) = fresh_db();
        let id = seed(&conn, "坏数据", &["m"]);
        conn.execute(
            "UPDATE custom_model_settings SET models = 'not-json' WHERE id = ?1",
            params![id],
        )
        .expect("写入坏数据");
        let row = get(&conn, id).expect("读取").expect("存在");
        assert!(row.models().is_empty(), "坏 JSON → 空列表，不炸链");
    }
}

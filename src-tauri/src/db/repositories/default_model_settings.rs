//! default_model_settings 表访问（内置免费额度模型）。
//!
//! 种子由 M6 迁移写入（智谱 / 硅基流动，凭据列为空占位）；内置模型的增删改
//! 属运营行为，不走应用内 UI（用户决策：内置不在 UI 暴露）。唯一写入口是
//! `set_api_key`，仅供启动时的 `builtin_credentials::sync` 同步构建期注入值。
//! `api_key` 明文只在本层与编排层之间流转，**禁止进日志/错误/DTO**。

use rusqlite::{Connection, params};

use super::parse_model_list;
use crate::error::AppError;

/// 内置模型行（`api_key` 为明文，仅内部使用）。
#[derive(Debug, Clone)]
pub struct DefaultModelRow {
    pub id: i64,
    pub name: String,
    /// 1=LLM, 2=语音, 3=图像, 4=视频, 5=翻译；编排只用 1。
    pub setting_type: i64,
    pub base_url: String,
    /// 模型名数组（库内 JSON 字符串形态）。
    pub models_json: String,
    pub api_key: String,
    /// 1=可用 / 2=停用（停用不参与编排）。
    pub is_can_use: i64,
}

impl DefaultModelRow {
    /// 模型名列表（解析失败 → 空 Vec，见 `super::parse_model_list`）。
    pub fn models(&self) -> Vec<String> {
        parse_model_list(&self.models_json)
    }

    /// 是否可用（`is_can_use = 1`）。
    pub fn is_available(&self) -> bool {
        self.is_can_use == 1
    }
}

/// 可用且类型为 LLM 的内置模型（编排链数据源；按 id 顺序）。
pub fn list_available_llm(conn: &Connection) -> Result<Vec<DefaultModelRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, type, base_url, models, api_key, is_can_use
             FROM default_model_settings
             WHERE is_can_use = 1 AND type = 1
             ORDER BY id ASC",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(DefaultModelRow {
                id: row.get(0)?,
                name: row.get(1)?,
                setting_type: row.get(2)?,
                base_url: row.get(3)?,
                models_json: row.get(4)?,
                api_key: row.get(5)?,
                is_can_use: row.get(6)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 同步内置模型 API Key（**唯一写入口**：构建期注入 → SQLCipher 库）。
///
/// 只在 `apply_migrations()` 的启动播种路径调用；未注入时写入空串，
/// 使已轮换作废的旧明文在存量库里也被清掉，内置额度自然降级为 BYOK。
/// 按平台名定位，不新增行——发布基线行数由 M6 的 `NOT EXISTS` 守卫固定。
pub fn set_api_key(conn: &Connection, name: &str, api_key: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE default_model_settings SET api_key = ?1 WHERE name = ?2",
        params![api_key, name],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 内置模型的**粘性轮询**起始下标（纯函数，可测）。
///
/// 语义（用户决策：内置轮询分摊，同时"尽量保持体感回复风格一致"）：
/// - 上次成功的模型仍在候选列表内 → **沿用**（风格不跳变）；
/// - 无历史 / 历史已失效 → 按轮询计数器取模（分摊负载）。
///
/// @param counter - 进程内请求计数器（每次编排 +1）
/// @param last_ok - 上次成功的内置模型下标（进程内记忆，None=无历史）
/// @param total - 本轮可用内置模型总数（0 → 返回 0，调用方按空链处理）
pub fn sticky_rotate(counter: usize, last_ok: Option<usize>, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    if let Some(i) = last_ok
        && i < total
    {
        return i; // 粘性：优先沿用上次成功的模型
    }
    counter % total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn list_available_llm_reads_m1_seeds_and_filters() {
        let (_dir, conn) = fresh_db();
        let rows = list_available_llm(&conn).expect("内置列表");
        assert_eq!(rows.len(), 2, "M1 种子：智谱 + 硅基流动");
        assert_eq!(rows[0].name, "智谱");
        assert_eq!(rows[0].models(), vec!["glm-4-flash"]);
        assert_eq!(rows[1].name, "硅基流动");
        assert_eq!(rows[1].models().len(), 6, "硅基流动 6 个模型");
        assert!(rows.iter().all(|r| r.is_available() && r.setting_type == 1));

        // 停用一条 → 不再出现
        conn.execute("UPDATE default_model_settings SET is_can_use = 2 WHERE name = '智谱'", [])
            .expect("停用");
        let rows = list_available_llm(&conn).expect("内置列表");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "硅基流动");
    }

    #[test]
    fn sticky_rotate_prefers_last_successful_model() {
        // 有历史且仍有效 → 粘性沿用（风格一致）
        assert_eq!(sticky_rotate(99, Some(1), 3), 1);
        // 历史失效（列表缩短）→ 回落轮询
        assert_eq!(sticky_rotate(2, Some(5), 3), 2);
        // 无历史 → 轮询取模
        assert_eq!(sticky_rotate(7, None, 3), 1);
        // 空链 → 0（调用方按空处理）
        assert_eq!(sticky_rotate(3, Some(0), 0), 0);
    }
}

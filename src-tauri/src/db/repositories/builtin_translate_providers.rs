//! builtin_translate_providers 表访问（内置翻译服务商：凭据 + 独立月度额度）。
//!
//! 红线：明文凭据仅在本层与 `translate/service` 之间流转，禁止进日志 / 错误 / DTO；
//! 对外视图只回 `has_credential` + 末 4 位指纹。SQL 全部参数化。
//!
//! M13 种子写入两家（百度 / 讯飞）；`name` 同时作为额度记账的 `platform` 键，
//! 沿用「百度翻译」以兼容 M12 起已产生的存量用量明细。

use rusqlite::{Connection, params};

use crate::error::AppError;

/// 内置服务商行（含明文凭据，仅内部使用，不参与序列化）。
#[derive(Debug, Clone)]
pub struct BuiltinProviderRow {
    pub id: i64,
    pub provider_id: String,
    pub name: String,
    pub app_id: String,
    pub api_secret: String,
    pub api_key: String,
    pub monthly_granted: i64,
    pub enabled: bool,
    pub sort_order: i64,
}

impl BuiltinProviderRow {
    /// 凭据是否齐全：百度需 APPID + 密钥；讯飞还需 APIKey。
    pub fn has_credentials(&self) -> bool {
        if self.app_id.is_empty() || self.api_secret.is_empty() {
            return false;
        }
        match self.provider_id.as_str() {
            "baidu" => true,
            "xfyun" => !self.api_key.is_empty(),
            _ => false,
        }
    }

    /// 是否可作为选路候选（启用且凭据齐全）。
    pub fn is_usable(&self) -> bool {
        self.enabled && self.has_credentials()
    }

    /// 凭据末 4 位指纹（只读诊断视图脱敏展示；凭据均为 ASCII）。
    pub fn credential_fingerprint(&self) -> Option<String> {
        let key = if self.api_key.is_empty() { &self.api_secret } else { &self.api_key };
        if key.len() >= 4 { Some(key[key.len() - 4..].to_string()) } else { None }
    }
}

/// 按 sort_order 读取全部内置服务商（含停用行，供选路与只读诊断）。
pub fn list_all(conn: &Connection) -> Result<Vec<BuiltinProviderRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, provider_id, name, app_id, api_secret, api_key, \
                    monthly_granted, is_can_use, sort_order \
             FROM builtin_translate_providers ORDER BY sort_order ASC, id ASC",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(BuiltinProviderRow {
                id: r.get(0)?,
                provider_id: r.get(1)?,
                name: r.get(2)?,
                app_id: r.get(3)?,
                api_secret: r.get(4)?,
                api_key: r.get(5)?,
                monthly_granted: r.get(6)?,
                enabled: r.get::<_, i64>(7)? == 1,
                sort_order: r.get(8)?,
            })
        })
        .map_err(AppError::from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// 构建期环境变量轮换内置凭据（与 M12 的 `seed_builtin` 同口径）。
///
/// 无 env 注入时不调用，M13 种子即开箱可用；注入时覆盖对应行，不改启停与额度。
pub fn update_credentials(
    conn: &Connection,
    provider_id: &str,
    app_id: &str,
    api_secret: &str,
    api_key: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE builtin_translate_providers SET app_id = ?1, api_secret = ?2, api_key = ?3 \
         WHERE provider_id = ?4",
        params![app_id, api_secret, api_key, provider_id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// 底层调整启停与月度上限（凭据列只读，不受影响）。
///
/// 不通过 IPC 暴露：内置两家的配置由 M13 种子固定，前端只做按用量自动均衡。
pub fn update_config(
    conn: &Connection,
    id: i64,
    enabled: bool,
    monthly_granted: i64,
) -> Result<(), AppError> {
    if monthly_granted <= 0 {
        return Err(AppError::Config("月度额度必须为正数".into()));
    }
    let can_use = if enabled { 1_i64 } else { 2_i64 };
    let changed = conn
        .execute(
            "UPDATE builtin_translate_providers SET is_can_use = ?1, monthly_granted = ?2 \
             WHERE id = ?3",
            params![can_use, monthly_granted, id],
        )
        .map_err(AppError::from)?;
    if changed == 0 {
        return Err(AppError::Config("内置服务商不存在".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::fresh_db;

    #[test]
    fn seeded_rows_are_both_usable_with_separate_grants() {
        let (_dir, conn) = fresh_db();
        let rows = list_all(&conn).expect("读内置服务商");
        assert_eq!(rows.len(), 2, "M13 应种子两家");
        assert_eq!(rows[0].provider_id, "baidu");
        assert_eq!(rows[1].provider_id, "xfyun");
        assert!(rows.iter().all(BuiltinProviderRow::is_usable), "两家凭据都应齐全且启用");
        assert!(rows.iter().all(|r| r.monthly_granted > 0), "各有独立月度上限");
        assert!(rows.iter().all(|r| r.credential_fingerprint().is_some()));
    }

    #[test]
    fn xfyun_requires_all_three_credentials() {
        let (_dir, conn) = fresh_db();
        let mut row = list_all(&conn)
            .expect("读取")
            .into_iter()
            .find(|r| r.provider_id == "xfyun")
            .expect("讯飞行");
        row.api_key.clear();
        assert!(!row.has_credentials(), "缺 APIKey 不可用");
        row.api_key = "key".into();
        row.api_secret.clear();
        assert!(!row.has_credentials(), "缺 APISecret 不可用");
    }

    #[test]
    fn update_credentials_overwrites_only_target_row() {
        let (_dir, conn) = fresh_db();
        update_credentials(&conn, "baidu", "new-id", "new-secret", "").expect("轮换");
        let rows = list_all(&conn).expect("读回");
        let baidu = rows.iter().find(|r| r.provider_id == "baidu").expect("百度行");
        let xfyun = rows.iter().find(|r| r.provider_id == "xfyun").expect("讯飞行");
        assert_eq!(baidu.app_id, "new-id");
        assert_eq!(baidu.api_secret, "new-secret");
        assert_eq!(baidu.monthly_granted, crate::config::BUILTIN_TRANSLATE_MONTHLY_GRANTED);
        assert!(baidu.is_usable(), "轮换后凭据仍齐全");
        assert_ne!(xfyun.app_id, "new-id", "不得影响另一家");
    }

    #[test]
    fn update_config_changes_quota_but_not_credentials() {
        let (_dir, conn) = fresh_db();
        let id = list_all(&conn).expect("读取")[0].id;
        update_config(&conn, id, false, 1234).expect("保存");
        let row = list_all(&conn).expect("读回").into_iter().find(|r| r.id == id).expect("目标行");
        assert!(!row.enabled);
        assert_eq!(row.monthly_granted, 1234);
        assert!(row.has_credentials(), "不得改动凭据");
        assert!(update_config(&conn, id, true, 0).is_err(), "额度必须为正");
        assert!(update_config(&conn, 9999, true, 100).is_err(), "不存在的 id 应报错");
    }
}

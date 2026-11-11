//! 内置共享凭据的**唯一运行时写入口**（构建期注入 → SQLCipher 库）。
//!
//! 背景（2026-11-11）：M6 / M12 / M13 曾把生产凭据明文写进迁移 SQL，而迁移 SQL
//! 随仓库分发、又被 `include_str!` 编入二进制，等于公开。现改为种子只留空占位，
//! 真实值由 `build.rs` 从构建期环境变量注入，`apply_migrations()` 在**每次启动**
//! 调用本模块同步到三张表。
//!
//! 三条纪律：
//! - **无条件覆盖**：未注入也写空串，确保存量库里已轮换作废的旧明文被清掉，
//!   而不是留着一个会 401 的失效 Key；
//! - **只写内置列**：不碰用户自填的 BYOK 配置、开关、额度与排序；
//! - **不回显**：本模块不打印、不记录、不把凭据放进错误信息。
//!
//! 局限：环境变量值会编入二进制，客户端分发仍可被逆向提取。要真正对终端用户
//! 保密，须改为服务端代理或纯 BYOK（见 `.env.example` 说明）。

use rusqlite::Connection;

use super::{builtin_translate_providers, default_model_settings, translate_settings};
use crate::config;
use crate::error::AppError;

/// 把构建期注入的凭据同步到 M6 / M12 / M13 三处内置表（幂等，每次启动执行）。
///
/// 放在 `apply_migrations()` 迁移成功后的共同路径，使生产 `init()` 与发布守卫
/// `init_with_key()` 行为一致；全新库与存量库因此得到同一份凭据状态。
pub fn sync(conn: &Connection) -> Result<(), AppError> {
    // M6：内置免费额度 LLM（按平台名定位，行数仍由迁移的 NOT EXISTS 守卫固定为 2）
    default_model_settings::set_api_key(conn, "智谱", config::ZHIPU_API_KEY.unwrap_or(""))?;
    default_model_settings::set_api_key(
        conn,
        "硅基流动",
        config::SILICONFLOW_API_KEY.unwrap_or(""),
    )?;

    // M12：translate_settings 的历史内置列（当前生产选路不读，但发布基线要求有值口径）
    translate_settings::seed_builtin(
        conn,
        config::BAIDU_TRANSLATE_APPID,
        config::BAIDU_TRANSLATE_SECRET,
    )?;

    // M13：内置翻译选路的唯一凭据真源（百度 + 讯飞）
    builtin_translate_providers::update_credentials(
        conn,
        "baidu",
        config::BAIDU_TRANSLATE_APPID.unwrap_or(""),
        config::BAIDU_TRANSLATE_SECRET.unwrap_or(""),
        "",
    )?;
    builtin_translate_providers::update_credentials(
        conn,
        "xfyun",
        config::XFYUN_APPID.unwrap_or(""),
        config::XFYUN_API_SECRET.unwrap_or(""),
        config::XFYUN_API_KEY.unwrap_or(""),
    )?;
    Ok(())
}

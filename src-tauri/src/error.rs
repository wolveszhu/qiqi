//! 统一错误类型：全应用唯一错误出口（规范 §4 / code-standards §4）。
//!
//! 纪律：业务代码只返回 `AppError`，禁止裸 `unwrap/expect`（F1 将以 clippy lints 固化）；
//! 序列化结构与 `src/api/types.ts` 的 `AppError` 一一对应（接口契约 §1）。

use serde::Serialize;

/// 前端可分类处理的错误。`kind` 供前端 switch，`message` 供直接展示（不含敏感信息）。
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppError {
    /// 数据库/SQLCipher 错误（含 rusqlite 映射）。
    Db(String),
    /// LLM 管道错误（请求失败/回落耗尽/维度不匹配等）。
    Llm(String),
    /// 窗口系统调用失败（位置/穿透/透明度等）。
    Window(String),
    /// Win32 探针失败（空闲/前台进程读取失败）。
    Probe(String),
    /// 文件系统 I/O（密钥存取、迁移备份、db 目录创建）。
    Io(String),
    /// 配置类错误（校验失败、版本过高等用户可行动的失败）。
    Config(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, msg) = match self {
            Self::Db(m) => ("db", m),
            Self::Llm(m) => ("llm", m),
            Self::Window(m) => ("window", m),
            Self::Probe(m) => ("probe", m),
            Self::Io(m) => ("io", m),
            Self::Config(m) => ("config", m),
        };
        write!(f, "[{kind}] {msg}")
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Db(format!("JSON 解析失败: {e}"))
    }
}

impl From<rusqlite_migration::Error> for AppError {
    fn from(e: rusqlite_migration::Error) -> Self {
        Self::Db(format!("迁移失败: {e}"))
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self::Window(format!("Tauri 调用失败: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_kind_content_camel_case() {
        // rename_all="camelCase" 对邻接标签枚举的 tag 同样生效：Db → db
        let json = serde_json::to_string(&AppError::Db("boom".into())).expect("序列化");
        assert_eq!(json, r#"{"kind":"db","message":"boom"}"#);
    }

    #[test]
    fn display_prefixes_kind() {
        assert_eq!(AppError::Config("x".into()).to_string(), "[config] x");
    }

    #[test]
    fn maps_rusqlite_error() {
        let e: AppError = rusqlite::Error::InvalidQuery.into();
        assert!(matches!(e, AppError::Db(_)));
    }
}

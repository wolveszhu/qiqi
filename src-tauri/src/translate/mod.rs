//! 全局划词翻译领域层：服务商预设、翻译编排与内置额度记账。
//!
//! 边界：所有 HTTP 与签名在 Rust 侧完成（baseline §G）；凭据只在库内与请求之间流转，
//! 不进日志 / 错误 / 事件负载。命令层只做参数校验与转发，取词能力见 `probe/selection`。

pub mod provider;
pub mod service;

pub use provider::{Lang, TranslateProvider, TranslateRequest, TranslationOutcome};
pub use service::{execute, get_settings, retry, save_settings};

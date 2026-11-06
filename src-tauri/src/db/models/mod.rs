//! 数据模型：按职责分文件，统一从本模块重导出。
//!
//! - `rows`：SQL 行结构体（内部使用，不参与序列化）；
//! - `values`：`app_settings.value` 列的 JSON 存储结构（含旧库兼容读取）；
//! - `dto`：命令层入参 / 出参（serde camelCase，与 `src/api/types.ts` 对齐）。
//!
//! 纪律：对外路径恒为 `crate::db::models::X`。拆分或移动定义时只改本文件的
//! `pub use`，调用方零改动——这是本目录可自由重组织而不影响功能的边界。

mod dto;
mod rows;
mod values;

pub use dto::*;
pub use rows::*;
pub use values::*;

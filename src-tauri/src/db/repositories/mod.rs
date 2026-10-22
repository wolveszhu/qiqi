//! 数据访问层（规范 §4.1）：**只写参数化 SQL，不做业务决策、不 emit 事件**。
//!
//! 红线（rule.md §H / code-standards §5）：
//! - 全部查询走 `params![]`，禁止字符串拼接式 SQL（DoD 以 grep 该目录无拼接宏为准）；
//! - 错误一律 `AppError::Db`，不做业务级 catch（错误由命令层原样上抛）。

pub mod app_settings;
pub mod diaries;
pub mod facts;
pub mod messages;
pub mod pet_states;
pub mod summaries;
pub mod vec_memory;

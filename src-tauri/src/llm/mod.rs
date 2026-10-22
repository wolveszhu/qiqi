//! LLM 域：OpenAI 兼容管道（规范 §4.1）。C4 全部实装。
//!
//! - `client.rs`：trait + Reqwest 实现 + 手写 SSE 分帧 + Mock（下游单测桩）；
//! - `router.rs`：三档解析（内置额度/BYOK）+ 回落链（main→memory→fallback→本地话术）；
//! - `prompt.rs`：System Prompt 拼装（人设/等级/记忆，禁播报数值）——纯函数；
//! - `distill.rs`：异步沉淀（facts JSON 三态解析 + 向量写入 + 会话摘要）；
//!   C7 扩展锚位：禁忌判定/日记生成复用本管道。
//!
//! API Key 只在 Rust 侧流转（code-standards §5），日志禁打 Key 与 Prompt 全文。

pub mod client;
pub mod distill;
pub mod prompt;
pub mod router;

//! LLM 域：OpenAI 兼容管道（规范 §4.1）。C4 全部实装；C7 扩展日记/禁忌/重建。
//!
//! - `client.rs`：trait + Reqwest 实现 + 手写 SSE 分帧 + Mock（下游单测桩）；
//! - `router.rs`：三档解析（内置额度/BYOK）+ 回落链（main→memory→fallback→本地话术）
//!   + memory 档目标解析（蒸馏/embedding/日记/重建共用）；
//! - `prompt.rs`：System Prompt 拼装（人设/等级/记忆/傲娇，禁播报数值）——纯函数；
//! - `distill.rs`：异步沉淀（facts/摘要/共鸣/禁忌）+ 记忆编辑向量同步 + 全量重建；
//! - `diary.rs`：每日日记生成（00:05 定时 + 启动补漏 + 手动重生成）。
//!
//! API Key 只在 Rust 侧流转（code-standards §5），日志禁打 Key 与 Prompt 全文。

pub mod client;
pub mod diary;
pub mod distill;
pub mod prompt;
pub mod quota;
pub mod router;
pub mod web_search;

use serde::Serialize;

/// `proactive:show` 事件负载（C6 主动话术 / C7 禁忌情绪化短句共用管道；camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProactiveShow {
    /// 会话 id（前缀 `proactive-`，与用户对话隔离）。
    pub session_id: String,
    /// 话术文本（已定稿，前端仅做本地打字机动画）。
    pub text: String,
    /// 表情指令（happy / tsundere / angry），驱动渲染器 `setExpression`。
    pub mood: String,
}

//! LLM 域：OpenAI 兼容管道（规范 §4.1）。
//!
//! TODO(C4): `client.rs`（SSE 流解析 + embeddings）、`router.rs`（三档 main/memory/fallback 路由与回落链）、
//! `prompt.rs`（System Prompt 拼装：人设/等级/记忆/傲娇，禁播报数值）、
//! `distill.rs`（异步沉淀：事实/摘要/向量；C7 扩展禁忌判定、C6 慢回复基调）。
//! API Key 只在 Rust 侧流转（code-standards §5）。

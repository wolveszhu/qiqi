//! chat 命令域（薄层）：**B5 为 echo 桩**——收到消息原样分片回吐，用于验证
//! SSE 事件链路与前端消费；C4 执行时整体替换实现，**接口签名与事件名不变**（前端零改动）。

use tauri::{AppHandle, Emitter};

use crate::error::AppError;

/// 流式增量事件负载（与 C4 一致）。
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamDelta {
    session_id: String,
    delta: String,
}

/// 流结束事件负载（与 C4 一致）。
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamDone {
    session_id: String,
}

/// 发送对话（echo 桩：原文分 3 片回吐 + done）。
///
/// [C4 替换] 真实实现：混合检索 → Prompt → SSE 流式转发 → 结束异步蒸馏。
#[tauri::command]
pub fn chat_send_stream(session_id: String, text: String, app: AppHandle) -> Result<(), AppError> {
    let chunks = split_echo(&text);
    for delta in chunks {
        app.emit("chat:stream:delta", StreamDelta { session_id: session_id.clone(), delta })
            .map_err(|e| AppError::Llm(format!("emit chat:stream:delta 失败: {e}")))?;
    }
    app.emit("chat:stream:done", StreamDone { session_id })
        .map_err(|e| AppError::Llm(format!("emit chat:stream:done 失败: {e}")))?;
    Ok(())
}

/// 中止会话（占位：C4 引入 per-session 取消标志后生效）。
#[tauri::command]
pub fn chat_abort(session_id: String) -> Result<(), AppError> {
    // [C4 替换] 置位取消标志，中止 in-flight 请求
    let _ = session_id;
    Ok(())
}

/// 把原文切成 3 片（空消息也给一个 done 前的空片，保证链路可观测）。
fn split_echo(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let size = text.len().div_ceil(3);
    let mut out = Vec::with_capacity(3);
    let mut rest = text;
    while !rest.is_empty() && out.len() < 2 {
        // 按字符边界切，避免 UTF-8 截断（中文场景必须）
        let mut end = size.min(rest.len());
        while end < rest.len() && !rest.is_char_boundary(end) {
            end += 1;
        }
        let (head, tail) = rest.split_at(end);
        out.push(head.to_string());
        rest = tail;
    }
    out.push(rest.to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_echo_returns_three_chunks() {
        let out = split_echo("abcdefg");
        assert_eq!(out.len(), 3);
        assert_eq!(out.concat(), "abcdefg", "分片拼接应还原原文");
    }

    #[test]
    fn split_echo_never_breaks_utf8() {
        let out = split_echo("用户讨厌吃香菜");
        assert_eq!(out.concat(), "用户讨厌吃香菜");
        for chunk in &out {
            assert!(std::str::from_utf8(chunk.as_bytes()).is_ok());
        }
    }

    #[test]
    fn split_echo_empty_text_yields_one_empty_chunk() {
        assert_eq!(split_echo(""), vec![String::new()]);
    }
}

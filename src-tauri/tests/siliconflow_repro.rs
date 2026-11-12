//! 临时探针（复现后删除）：硅基流动 `Qwen/Qwen3.5-4B` SSE 流在 reqwest 下的
//! `error decoding response body`，以及「每帧带 usage 吞掉 delta」的行为观测。
//!
//! 运行（Key 走环境变量，绝不入代码/日志）：
//! `$env:QIQI_SILICONFLOW_API_KEY = <key>; cargo test --test siliconflow_repro -- --ignored --nocapture`

use futures_util::StreamExt;
use qiqi_lib::llm::client::{
    ChatMessage, ChatRequest, LlmClient, LlmTarget, ReqwestLlmClient, StreamEvent,
};

const MODEL: &str = "Qwen/Qwen3.5-4B";
const BASE: &str = "https://api.siliconflow.cn/v1";

fn request() -> ChatRequest {
    ChatRequest {
        messages: vec![ChatMessage { role: "user".into(), content: "用50字介绍自己".into() }],
        max_tokens: Some(200),
    }
}

fn target(api_key: String) -> LlmTarget {
    LlmTarget {
        tier: "builtin",
        platform: String::new(),
        base_url: BASE.into(),
        api_key,
        model: MODEL.into(),
    }
}

/// 探针 1：项目实现（ReqwestLlmClient，默认协议协商含 h2）。
#[tokio::test]
#[ignore = "手动探针：需 QIQI_SILICONFLOW_API_KEY 与外网"]
async fn client_impl_stream() {
    let key = std::env::var("QIQI_SILICONFLOW_API_KEY").expect("设 QIQI_SILICONFLOW_API_KEY");
    let client = ReqwestLlmClient::new(1536);
    let stream = client.chat_stream(&target(key), request()).await.expect("建立流");
    let mut stream = std::pin::pin!(stream);
    let (mut deltas, mut usages, mut chars) = (0usize, 0usize, 0usize);
    while let Some(event) = stream.next().await {
        match event {
            Ok(StreamEvent::Delta(d)) => {
                deltas += 1;
                chars += d.chars().count();
            },
            Ok(StreamEvent::Usage(_)) => usages += 1,
            Ok(StreamEvent::Done) => println!("[client_impl] DONE"),
            Err(e) => panic!("[client_impl] 流错误: {e}"),
        }
    }
    println!("[client_impl] deltas={deltas} usages={usages} chars={chars}");
}

/// 探针 2：裸 reqwest + `http1_only`（对照：验证是否 h2 协商导致解码失败）。
#[tokio::test]
#[ignore = "手动探针：需 QIQI_SILICONFLOW_API_KEY 与外网"]
async fn raw_http1_stream() {
    let key = std::env::var("QIQI_SILICONFLOW_API_KEY").expect("设 QIQI_SILICONFLOW_API_KEY");
    let http = reqwest::Client::builder().http1_only().no_proxy().build().unwrap();
    let body = serde_json::json!({
        "model": MODEL,
        "messages": [{ "role": "user", "content": "用50字介绍自己" }],
        "stream": true,
        "stream_options": { "include_usage": true },
    });
    let resp = http
        .post(format!("{BASE}/chat/completions"))
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .expect("发送");
    println!("[http1] status={} version={:?}", resp.status(), resp.version());
    let mut bytes = resp.bytes_stream();
    let mut chunks = 0usize;
    while let Some(chunk) = bytes.next().await {
        match chunk {
            Ok(_) => chunks += 1,
            Err(e) => panic!("[http1] 流错误: {e}"),
        }
    }
    println!("[http1] chunks={chunks}");
}

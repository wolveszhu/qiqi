//! 构建期凭据注入：**内置共享凭据的唯一入口**。
//!
//! 迁移 SQL 只保留空占位（表结构 / 行数 / 幂等守卫不变），真实值在此注入并编入
//! 二进制，启动路径 `builtin_credentials::sync` 写入 SQLCipher 库。明文因此不进
//! 仓库、不进 git 历史、不进前端产物、不进日志。
//!
//! 取值优先级：① 进程环境变量（GitHub Actions secrets / 发布机显式 export）；
//! ② 仓库根 `.env`（本地开发，已被 `.gitignore` 忽略）。两者都没有时
//! `option_env!` 得到 None → 启动播种写空串 → 内置通道降级为「未配置」，
//! 用户自填 BYOK 不受影响。
//!
//! 声明 `rerun-if-changed=../.env` 与 `rerun-if-env-changed`，保证**密钥轮换后
//! 自动重编译**——否则 rustc 会复用编入旧 Key 的产物，改了 `.env` 也不生效。
//! 变量清单与 `config.rs` / `.env.example` / `release.yml` 保持同步。
//!
//! 注意：环境变量值会编入二进制，客户端分发场景下**仍可被逆向提取**——这是
//! "避免源码仓库泄露"的方案，不是"凭据对终端用户保密"的方案。要真正保密须改
//! 服务端代理或纯 BYOK。

use std::collections::HashMap;

/// 允许注入的构建期凭据变量（改动此处必须同步 config.rs、.env.example、release.yml）。
const CREDENTIAL_ENV: &[&str] = &[
    // 内置额度 LLM（M6）
    "QIQI_ZHIPU_API_KEY",
    "QIQI_SILICONFLOW_API_KEY",
    // 内置翻译 · 百度（M12 / M13）
    "QIQI_BAIDU_APPID",
    "QIQI_BAIDU_SECRET",
    // 内置翻译 · 讯飞（M13）
    "QIQI_XFYUN_APPID",
    "QIQI_XFYUN_API_SECRET",
    "QIQI_XFYUN_API_KEY",
];

/// 从仓库根 `.env` 读取白名单凭据（build script 的 CWD 是 crate 根 `src-tauri/`）。
///
/// 只认 [`CREDENTIAL_ENV`] 白名单，避免把 `.env` 里的其它本地配置误编进二进制；
/// 返回集合而不调用 `set_var`：Rust 2024 的 `set_var` 是 `unsafe`，且无需污染
/// 构建进程环境，直接输出 `cargo:rustc-env` 即可。
fn read_dotenv() -> HashMap<String, String> {
    const DOTENV: &str = "../.env";
    // 轮换后改 .env 必须触发重编译；文件从无到有也会触发。
    println!("cargo:rerun-if-changed={DOTENV}");
    let Ok(text) = std::fs::read_to_string(DOTENV) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for line in text.lines() {
        // PowerShell 5.1 写出的 .env 可能带 UTF-8 BOM，逐行去掉以免首行变量名失配。
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim().trim_start_matches("export ").trim();
        if !CREDENTIAL_ENV.contains(&key) {
            continue;
        }
        let value = raw_value.trim().trim_matches('"').trim_matches('\'').trim();
        if !value.is_empty() {
            out.insert(key.to_string(), value.to_string());
        }
    }
    out
}

fn main() {
    let dotenv = read_dotenv();
    for name in CREDENTIAL_ENV {
        println!("cargo:rerun-if-env-changed={name}");
        // 进程环境优先：CI 用 GitHub Actions secrets，发布机可显式 export 覆盖 .env。
        let value = std::env::var(name).ok().or_else(|| dotenv.get(*name).cloned());
        if let Some(value) = value {
            println!("cargo:rustc-env={name}={value}");
        }
    }
    tauri_build::build()
}

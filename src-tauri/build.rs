//! 构建期凭据注入：**内置共享凭据的唯一入口**。
//!
//! 迁移 SQL 只保留空占位（表结构 / 行数 / 幂等守卫不变），真实值在 CI / 发布机上
//! 由环境变量注入并编译进二进制，启动路径 `builtin_credentials::sync` 写入 SQLCipher 库。
//! 明文因此不进仓库、不进 git 历史、不进前端产物、不进日志。
//!
//! 注意：环境变量值会编入二进制，客户端分发场景下**仍可被逆向提取**——这是
//! "避免源码仓库泄露"的方案，不是"凭据对终端用户保密"的方案。要真正保密须改
//! 服务端代理或纯 BYOK。变量清单与 `.env.example` / CI secrets 保持同步。

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

fn main() {
    // 未注入时不输出 rustc-env，Rust 侧 `option_env!` 得到 None → 写入空串，
    // 内置通道降级为「未配置」（用户自填 BYOK 不受影响）。
    // rerun-if-env-changed 保证轮换凭据后触发重编译，否则 rustc 会复用旧产物。
    for name in CREDENTIAL_ENV {
        println!("cargo:rerun-if-env-changed={name}");
        if let Ok(value) = std::env::var(name) {
            println!("cargo:rustc-env={name}={value}");
        }
    }
    tauri_build::build()
}

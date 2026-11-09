fn main() {
    // 划词翻译内置凭据：只从构建期环境变量读取，明文不进仓库 / 源码 / 日志。
    // 未注入时不输出 rustc-env，Rust 侧 `option_env!` 得到 None，内置通道降级为「未配置」；
    // rerun-if-env-changed 保证轮换凭据后触发重编译（否则 rustc 会复用旧产物）。
    for name in ["QIQI_BAIDU_APPID", "QIQI_BAIDU_SECRET"] {
        println!("cargo:rerun-if-env-changed={name}");
        if let Ok(value) = std::env::var(name) {
            println!("cargo:rustc-env={name}={value}");
        }
    }
    tauri_build::build()
}

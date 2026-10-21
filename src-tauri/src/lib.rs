//! 应用组装根：插件注册、命令注册、启动迁移与后台任务（结构见《项目目录结构规范》§4）。
//!
//! 边界：本文件只做"组装"——插件初始化、invoke_handler 注册、setup 钩子接线；
//! 业务规则一律放领域模块（db/llm/affection/proactive/probe/window），命令薄层放 commands/。

mod affection;
mod db;
mod llm;
mod proactive;
mod probe;
mod window;

pub mod commands;
pub mod config;
pub mod error;
pub mod state;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// TODO(B1): 窗口 B 关闭拦截（CloseRequested → prevent_close + hide）
// TODO(B4): setup 内 db::connection::init + app.manage(AppState) + 启动衰减结算
// TODO(C2): 插件 global-shortcut/autostart + 托盘 + probe 轮询线程
// TODO(B5): invoke_handler 追加六域命令

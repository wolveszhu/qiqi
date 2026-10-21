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
        .on_window_event(|window, event| {
            // B1.2 关闭=hide 拦截（策略与理由见 window/manager.rs 文档）：
            // A/B 均拦——桌宠常驻，退出统一走托盘（C2.6 app.exit）。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window::manager::hide_window(window);
            }
        })
        .invoke_handler(tauri::generate_handler![greet, commands::window::window_hide])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// TODO(B1): 窗口 B visible 开发期策略——C2.6 托盘到位后改 conf visible:false
//
// 组装锚位（B3.4）：后续任务只允许在以下锚位接线，保持组装点唯一——
// [B4] .setup(): db::connection::init(&db_path)? + app.manage(AppState) + 启动衰减结算（C5）
// [C2] .plugin(): global-shortcut / autostart + 托盘（tray.rs）+ probe 轮询线程（100ms 单点）
// [B5] .invoke_handler(): 追加六域命令（settings/pet/chat/memory/probe/window 全量组）
// [C6] .setup(): 蒸馏/日记定时任务 spawn（C7 扩展禁忌判定）

//! 应用组装根：插件注册、命令注册、启动迁移与后台任务（结构见《项目目录结构规范》§4）。
//!
//! 边界：本文件只做"组装"——插件初始化、invoke_handler 注册、setup 钩子接线；
//! 业务规则一律放领域模块（db/llm/affection/proactive/probe/window），命令薄层放 commands/。

mod affection;
mod llm;
mod proactive;
mod probe;
mod window;

pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod state;

// greet（脚手架遗留）已按 B3 决策移除：命令统一从 commands/ 出口，不留死代码。

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
        .setup(|app| {
            // [B4] 数据层接线：SQLCipher 加密连接 + 迁移链 + 状态注册。
            // 失败即启动失败（fail-fast：无库不应启动桌宠，B4 子计划决策）。
            use tauri::Manager as _;
            let db_path = app
                .path()
                .app_data_dir()
                .map_err(|e| error::AppError::Io(format!("解析 app_data_dir 失败: {e}")))?
                .join("pet.db");
            let db::Db { conn } = db::init(&db_path)?;
            app.manage(crate::state::AppState { db: conn });
            Ok(())
        })
        // [B5] 命令注册：window 组（B1 临时通道）→ C2 扩展；settings/pet/chat/memory 组本计划落位。
        // 应用自定义命令**免 capability 授权**（B2 勘误#1）。
        .invoke_handler(tauri::generate_handler![
            commands::window::window_hide,
            commands::settings::settings_get_typed,
            commands::settings::settings_save_typed,
            commands::pet::pet_get_state,
            commands::pet::pet_update_state,
            commands::chat::chat_send_stream,
            commands::chat::chat_abort,
            commands::memory::memory_list_facts,
            commands::memory::memory_list_diaries,
            commands::memory::memory_update_fact,
            commands::memory::memory_delete_fact,
            commands::memory::memory_regenerate_diary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// TODO(B1): 窗口 B visible 开发期策略——C2.6 托盘到位后改 conf visible:false
//
// 组装锚位（B3.4）：后续任务只允许在以下锚位接线，保持组装点唯一——
// [B4] .setup(): 已接线（db::init + app.manage）；[C5] 同锚位追加启动衰减结算
// [C2] .plugin(): global-shortcut / autostart + 托盘（tray.rs）+ probe 轮询线程（100ms 单点）
// [B5] .invoke_handler(): 追加六域命令（settings/pet/chat/memory/probe/window 全量组）
// [C6] .setup(): 蒸馏/日记定时任务 spawn（C7 扩展禁忌判定）

//! 应用组装根：插件注册、命令注册、启动迁移与后台任务（结构见《项目目录结构规范》§4）。
//!
//! 边界：本文件只做"组装"——插件初始化、invoke_handler 注册、setup 钩子接线；
//! 业务规则一律放领域模块（db/llm/affection/proactive/probe/window），命令薄层放 commands/。

// F1.4：测试代码（#[cfg(test)] 模块与 tests/ 集成测试）对 fixture 用 unwrap/expect 属合理用法，
// 统一豁免四项 clippy lint，避免门禁误伤；lib 非测试代码仍强制。
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)
)]

pub mod affection;
pub mod llm;
mod proactive;
mod probe;
mod window;

pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod state;

#[cfg(test)]
mod test_util;

use crate::state::AppState;

// greet（脚手架遗留）已按 B3 决策移除：命令统一从 commands/ 出口，不留死代码。

#[cfg_attr(mobile, tauri::mobile_entry_point)]
// F1.4：Tauri 启动装配的 `expect` 为启动期致命项错误、无恢复路径，豁免 expect_used。
#[allow(clippy::expect_used)]
pub fn run() {
    let shortcuts = tauri_plugin_global_shortcut::Builder::new()
        // Ctrl+Shift+P：唤起对话输入（D1 消费 `window:chat:toggle`）
        .with_shortcut("CommandOrControl+Shift+P")
        .expect("注册快捷键 Ctrl+Shift+P 失败（可能与其他软件冲突）")
        .with_handler(|app, _shortcut, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                use tauri::Emitter as _;
                let _ = app.emit("window:chat:toggle", ());
            }
        })
        // Ctrl+Shift+H：窗口 A 显隐切换（C2.7）
        .with_shortcut("CommandOrControl+Shift+H")
        .expect("注册快捷键 Ctrl+Shift+H 失败（可能与其他软件冲突）")
        .with_handler(|app, _shortcut, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                toggle_main_window(app);
            }
        })
        .build();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(shortcuts)
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .on_window_event(|window, event| {
            // B1.2 关闭=hide 拦截（策略与理由见 window/manager.rs 文档）：
            // A/B 均拦——桌宠常驻，退出统一走托盘（C2.6 app.exit）。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window::manager::hide_window(window);
                // 隐藏时复位穿透（避免隐藏态残留"不穿透"，恢复显示时误判）
                let _ = window::click_through::reset_to_passthrough(window);
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
            app.manage(crate::state::AppState::new(conn));

            // [C2] 桌面融合装配：托盘 + 探针单点轮询 + 穿透同步循环
            window::tray::build_tray(app)?;
            let handle = app.handle().clone();
            probe::scheduler::start_cursor_probe(handle.clone());
            start_click_through_sync(handle.clone());

            // [C5] 启动衰减结算：异步、失败仅 log 不阻断窗口（幂等）
            {
                use tauri::Manager as _;
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    if let Ok(conn) = state.db.lock()
                        && let Err(e) = crate::affection::settle_decay_at_startup(&app_handle, &conn)
                    {
                        eprintln!("[affection] 启动衰减结算失败: {e}");
                    }
                });
            }

            // [C7] 日记调度：启动补漏（近 7 天）+ 每日 00:05 生成昨日日记（单例 spawn，失败仅 log）
            {
                let app_handle = app.handle().clone();
                crate::llm::diary::spawn_scheduler(app_handle);
            }
            Ok(())
        })
        // [B5] 命令注册：settings/pet/chat/memory 组（B5）；window/probe 组（C2 全量）。
        // 应用自定义命令**免 capability 授权**（B2 勘误#1）。
        .invoke_handler(tauri::generate_handler![
            commands::window::window_hide,
            commands::window::window_show,
            commands::window::window_start_drag,
            commands::window::window_set_ignore_cursor_events,
            commands::window::window_get_outer_position,
            commands::window::window_snap_to_edge,
            commands::window::window_restore_from_snap,
            commands::window::window_set_hit_region,
            commands::window::window_set_bubble_mode,
            commands::window::window_set_sleep_mode,
            commands::window::window_set_click_through_enabled,
            commands::settings::settings_get_typed,
            commands::settings::settings_save_typed,
            commands::settings::settings_save_dnd,
            commands::settings::settings_list_providers,
            commands::settings::settings_set_quota_mode,
            commands::settings::settings_set_snap_sleep,
            commands::settings::settings_list_custom_models,
            commands::settings::settings_save_custom_model,
            commands::settings::settings_delete_custom_model,
            commands::settings::settings_move_custom_model,
            commands::pet::pet_get_state,
            commands::pet::pet_update_state,
            commands::pet::pet_settle_affection,
            commands::pet::pet_list_affection_logs,
            commands::chat::chat_send_stream,
            commands::chat::chat_abort,
            commands::chat::chat_list_sessions,
            commands::chat::chat_get_messages,
            commands::chat::chat_delete_session,
            commands::chat::chat_resolve_dropped_files,
            commands::memory::memory_list_facts,
            commands::memory::memory_list_diaries,
            commands::memory::memory_update_fact,
            commands::memory::memory_delete_fact,
            commands::memory::memory_add_fact,
            commands::memory::memory_regenerate_diary,
            commands::probe::probe_get_cursor,
            commands::probe::probe_get_snapshot,
            commands::probe::proactive_generate,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 穿透同步循环（C2.4）：独立线程按探针间隔驱动，复用 cursor 缓存与状态机。
/// 与 cursor 探针**同一节奏**（100ms），零额外定时器。
fn start_click_through_sync(app: tauri::AppHandle) {
    use tauri::Manager as _;
    std::thread::spawn(move || {
        loop {
            if let Some(window) = app.get_webview_window(window::manager::LABEL_MAIN) {
                // 窗口隐藏时不做穿透切换（避免无谓系统调用）
                if window.is_visible().unwrap_or(false) {
                    let _ = window::click_through::sync_click_through(&app, &window);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(config::PROBE_INTERVAL_MS));
        }
    });
}

/// 快捷键处理：窗口 A 显隐切换。
fn toggle_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager as _;
    if let Some(window) = app.get_webview_window(window::manager::LABEL_MAIN) {
        match window.is_visible() {
            Ok(true) => {
                let _ = window.hide();
            },
            Ok(false) => {
                let _ = window.show();
                let _ = window.set_focus();
            },
            Err(_) => {},
        }
    }
}

// TODO(B1): 窗口 B visible 开发期策略——C2.6 托盘到位后改 conf visible:false
//
// 组装锚位（B3.4）：后续任务只允许在以下锚位接线，保持组装点唯一——
// [B4] .setup(): 已接线（db::init + app.manage）；[C5] 同锚位追加启动衰减结算
// [C2] .plugin(): global-shortcut / autostart + 托盘（tray.rs）+ probe 轮询线程（100ms 单点）
// [B5] .invoke_handler(): 追加六域命令（settings/pet/chat/memory/probe/window 全量组）
// [C6] .setup(): 蒸馏/日记定时任务 spawn（C7 扩展禁忌判定）

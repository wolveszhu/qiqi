//! 托盘菜单（C2.6）：窗口 A 无任务栏入口，托盘是唯一常驻出口。
//!
//! 菜单：显示/隐藏（窗口 A）、设置（显示窗口 B）、开机自启开关、退出。
//! 退出走 Rust 侧 `app.exit(0)`——绕过 `on_window_event` 的关闭拦截（B1.2）。

use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Manager, Runtime};

use super::manager::{LABEL_MAIN, LABEL_SETTINGS};
use crate::error::AppError;

/// 菜单项 id（集中常量，禁魔法字符串）。
const MENU_SHOW: &str = "tray_show";
const MENU_HIDE: &str = "tray_hide";
const MENU_SETTINGS: &str = "tray_settings";
const MENU_AUTOSTART: &str = "tray_autostart";
const MENU_QUIT: &str = "tray_quit";

/// 托盘图标（编译期嵌入，Tauri Image 仅支持 PNG/ICO；32×32 托盘原生尺寸）。
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../icons/tray.png");

/// 构建托盘并挂接菜单事件（setup 内调用一次）。
pub fn build_tray(app: &App) -> Result<(), AppError> {
    let menu = MenuBuilder::new(app)
        .items(&[
            &MenuItemBuilder::with_id(MENU_SHOW, "显示桌宠").build(app)?,
            &MenuItemBuilder::with_id(MENU_HIDE, "隐藏桌宠").build(app)?,
            &MenuItemBuilder::with_id(MENU_SETTINGS, "设置").build(app)?,
            &MenuItemBuilder::with_id(MENU_AUTOSTART, "开机自启").build(app)?,
            &MenuItemBuilder::with_id(MENU_QUIT, "退出").build(app)?,
        ])
        .build()
        .map_err(AppError::from)?;

    let icon = Image::from_bytes(TRAY_ICON_PNG)
        .map_err(|e| AppError::Window(format!("托盘图标解码失败: {e}")))?;
    TrayIconBuilder::with_id("qiqi-main")
        .icon(icon)
        .tooltip("Qiqi")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .build(app)
        .map_err(AppError::from)?;
    Ok(())
}

/// 菜单事件分发（动作分发集中在单函数，便于核对权限与副作用）。
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        MENU_SHOW => show_window(app, LABEL_MAIN),
        MENU_HIDE => {
            if let Some(w) = app.get_webview_window(LABEL_MAIN) {
                let _ = w.hide();
            }
        },
        MENU_SETTINGS => show_window(app, LABEL_SETTINGS),
        MENU_AUTOSTART => toggle_autostart(app),
        MENU_QUIT => app.exit(0),
        _ => {},
    }
}

/// 显示并聚焦指定窗口（托盘唤起的统一入口）。
fn show_window<R: Runtime>(app: &AppHandle<R>, label: &str) {
    use tauri::Manager as _;
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// 开机自启开关（读当前状态取反；失败静默——菜单不因注册表异常崩溃）。
fn toggle_autostart<R: Runtime>(app: &AppHandle<R>) {
    use tauri_plugin_autostart::ManagerExt as _;
    let manager = app.autolaunch();
    match manager.is_enabled() {
        Ok(true) => {
            let _ = manager.disable();
        },
        Ok(false) => {
            let _ = manager.enable();
        },
        Err(_) => {},
    }
}

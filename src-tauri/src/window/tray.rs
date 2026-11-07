//! 托盘交互（C2.6）：窗口 A 无任务栏入口，托盘是唯一常驻出口。
//!
//! 交互：左键 → 唤起应用（桌宠回桌面 **且** 打开设置窗口）；右键 → 弹出菜单
//! （显示/隐藏桌宠、设置、开机自启开关、退出）。
//! 退出走 Rust 侧 `app.exit(0)`——绕过 `on_window_event` 的关闭拦截（B1.2）。

use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, Runtime};

use super::manager::{LABEL_MAIN, LABEL_SETTINGS};
use crate::error::AppError;
use crate::i18n::{TrayStrings, tray_strings};

/// 菜单项 id（集中常量，禁魔法字符串）。
const MENU_SHOW: &str = "tray_show";
const MENU_HIDE: &str = "tray_hide";
const MENU_SETTINGS: &str = "tray_settings";
const MENU_AUTOSTART: &str = "tray_autostart";
const MENU_QUIT: &str = "tray_quit";

/// 托盘图标（编译期嵌入，Tauri Image 仅支持 PNG/ICO；32×32 托盘原生尺寸）。
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../icons/tray.png");

/// 构建托盘并挂接菜单/点击事件（setup 内调用一次）。
///
/// 菜单文案按 `locale` 本地化（托盘由 OS 直接绘制，不进 WebView）。
/// 左键打开设置窗口：捕获 `AppHandle`（闭包需 `'static`，不能借 `&App`）；
/// 右键弹菜单（`show_menu_on_left_click(false)` 让左键不再弹菜单，菜单默认在右键弹出）。
pub fn build_tray(app: &App, locale: &str) -> Result<(), AppError> {
    let strings = tray_strings(locale);
    let menu = build_menu(app, &strings)?;

    let icon = Image::from_bytes(TRAY_ICON_PNG)
        .map_err(|e| AppError::Window(format!("托盘图标解码失败: {e}")))?;

    // 左键点击回调持有 AppHandle（'static），用于唤起应用（桌宠 + 设置窗口）。
    // 注意 `app.app_handle()` 返回 `&AppHandle`（借用，受 `app` 生命周期约束），
    // 闭包需 'static，故 clone 出独立句柄。
    let handle = app.handle().clone();
    TrayIconBuilder::with_id("qiqi-main")
        .icon(icon)
        .tooltip("Qiqi")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(move |_icon, event| {
            // 左键 = 唤起应用（桌宠回桌面 + 打开设置）；右键交给菜单，其余按键忽略。
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                reveal_app(&handle);
            }
        })
        .build(app)
        .map_err(AppError::from)?;
    Ok(())
}

/// 按本地化文案构建托盘菜单（Rust 侧菜单 API 要求 `MenuItem` 生命周期 ≤ manager）。
///
/// 抽为独立函数：首次构建（`build_tray`）与运行时重建（`rebuild_tray_menu`）共用同一份文案来源。
fn build_menu<M: tauri::Manager<R>, R: tauri::Runtime>(
    manager: &M,
    s: &TrayStrings,
) -> Result<tauri::menu::Menu<R>, AppError> {
    MenuBuilder::new(manager)
        .items(&[
            &MenuItemBuilder::with_id(MENU_SHOW, s.show).build(manager)?,
            &MenuItemBuilder::with_id(MENU_HIDE, s.hide).build(manager)?,
            &MenuItemBuilder::with_id(MENU_SETTINGS, s.settings).build(manager)?,
            &MenuItemBuilder::with_id(MENU_AUTOSTART, s.autostart).build(manager)?,
            &MenuItemBuilder::with_id(MENU_QUIT, s.quit).build(manager)?,
        ])
        .build()
        .map_err(AppError::from)
}

/// 运行时重建托盘菜单（语言切换后立即生效；`set_locale` 命令调用）。
///
/// 取已注册的托盘图标（`qiqi-main`）并替换其菜单；托盘不存在时静默（不致命）。
pub fn rebuild_tray_menu<R: Runtime>(app: &AppHandle<R>, locale: &str) -> Result<(), AppError> {
    let strings = tray_strings(locale);
    let menu = build_menu(app, &strings)?;
    if let Some(tray) = app.tray_by_id("qiqi-main") {
        tray.set_menu(Some(menu)).map_err(AppError::from)?;
    }
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

/// 托盘左键语义：**唤起整个应用**——桌宠（窗口 A）回到桌面 + 打开设置窗口（窗口 B）。
///
/// 顺序固定：先显示桌宠（不抢焦点），再显示并聚焦设置窗口，
/// 保证最终焦点落在设置窗口（桌宠是 alwaysOnTop 透明层，后聚焦会盖住输入框焦点）。
fn reveal_app<R: Runtime>(app: &AppHandle<R>) {
    reveal_window(app, LABEL_MAIN, false);
    reveal_window(app, LABEL_SETTINGS, true);
}

/// 显示并聚焦指定窗口（托盘菜单「显示桌宠 / 设置」的统一入口）。
fn show_window<R: Runtime>(app: &AppHandle<R>, label: &str) {
    reveal_window(app, label, true);
}

/// 窗口还原的单一实现：最小化 → `unminimize()`，隐藏 → `show()`，可选聚焦。
///
/// **为什么显式 unminimize**：窗口 B 缩到任务栏后仅调 `show()` 不保证还原到桌面
/// （底层 tao 对最小化窗口的 show 行为随平台而异），故先判最小化态再还原，
/// 让「托盘左键 / 菜单」两类唤起在最小化态下都能把应用带回桌面。
fn reveal_window<R: Runtime>(app: &AppHandle<R>, label: &str, focus: bool) {
    use tauri::Manager as _;
    let Some(w) = app.get_webview_window(label) else {
        return;
    };
    if w.is_minimized().unwrap_or(false) {
        let _ = w.unminimize();
    }
    let _ = w.show();
    if focus {
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

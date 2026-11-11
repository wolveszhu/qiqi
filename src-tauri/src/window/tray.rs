//! 托盘交互（C2.6）：窗口 A 无任务栏入口，托盘是唯一常驻出口。
//!
//! 交互：左键 → 唤起应用（桌宠回桌面 **且** 打开设置窗口）；右键 → 弹出菜单
//! （显示/隐藏桌宠、设置、开机自启开关、退出）。
//! 退出走 Rust 侧 `app.exit(0)`——绕过 `on_window_event` 的关闭拦截（B1.2）。

use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt as _;

use super::click_through;
use super::manager::{LABEL_MAIN, LABEL_SETTINGS};
use crate::error::AppError;
use crate::i18n::{TrayStrings, tray_strings};
use crate::state::AppState;

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

    // 开发态清理：自动关闭残留的 dev 开机自启（详见 `prune_dev_autostart`）。
    // 注意 `handle` 已 move 进左键闭包，这里改用 `app.handle()` 取一份新句柄。
    #[cfg(debug_assertions)]
    prune_dev_autostart(&app.handle().clone());

    Ok(())
}

/// 按本地化文案构建托盘菜单（Rust 侧菜单 API 要求 `MenuItem` 生命周期 ≤ manager）。
///
/// 抽为独立函数：首次构建（`build_tray`）与运行时重建（`rebuild_tray_menu`）共用同一份文案来源。
fn build_menu<M: tauri::Manager<R>, R: tauri::Runtime>(
    manager: &M,
    s: &TrayStrings,
) -> Result<tauri::menu::Menu<R>, AppError> {
    // 开机自启文案随真实开关态翻转：已开启 → 显示「关闭开机自启」，反之「开机自启」。
    let autostart_label = match manager.autolaunch().is_enabled() {
        Ok(true) => s.autostart_on,
        _ => s.autostart,
    };
    MenuBuilder::new(manager)
        .items(&[
            &MenuItemBuilder::with_id(MENU_SHOW, s.show).build(manager)?,
            &MenuItemBuilder::with_id(MENU_HIDE, s.hide).build(manager)?,
            &MenuItemBuilder::with_id(MENU_SETTINGS, s.settings).build(manager)?,
            &MenuItemBuilder::with_id(MENU_AUTOSTART, autostart_label).build(manager)?,
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
        MENU_AUTOSTART => {
            toggle_autostart(app);
            // 切换后立即重建菜单，使标签在「开机自启 ↔ 关闭开机自启」间即时翻转
            let locale = app
                .state::<AppState>()
                .locale
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default();
            let _ = rebuild_tray_menu(app, &locale);
        },
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
///
/// **为什么显式复位穿透**：`CloseRequested` 拦截隐藏窗口时曾对 A/B 一律置穿透，
/// 而 `show()` 不清除该标志，导致「关闭设置窗口 → 托盘重新打开」后整窗点不动。
fn reveal_window<R: Runtime>(app: &AppHandle<R>, label: &str, focus: bool) {
    use tauri::Manager as _;
    let Some(w) = app.get_webview_window(label) else {
        return;
    };
    if w.is_minimized().unwrap_or(false) {
        let _ = w.unminimize();
    }
    let _ = click_through::ensure_interactive(&w);
    let _ = w.show();
    if focus {
        let _ = w.set_focus();
    }
}

/// 开机自启开关（读当前状态取反；失败静默——菜单不因注册表异常崩溃）。
///
/// 开发构建（`cfg!(debug_assertions)`）下**禁止启用**：dev 二进制加载 `devUrl`
/// （localhost:1420 开发服务器），系统开机自启时并无 vite 在跑，会导致
/// 「无法连接本地网络」。故仅允许关闭，启用一律拒绝并提示。
fn toggle_autostart<R: Runtime>(app: &AppHandle<R>) {
    let manager = app.autolaunch();
    match manager.is_enabled() {
        Ok(true) => {
            let _ = manager.disable();
        },
        Ok(false) => {
            if cfg!(debug_assertions) {
                emit_autostart_blocked_toast(app);
                return;
            }
            let _ = manager.enable();
        },
        Err(_) => {},
    }
}

/// 启动期清理：开发构建下若已残留开机自启注册（dev 二进制无法脱离 vite 运行），
/// 自动关闭它以消除「开机连不上 localhost:1420」的故障，并提示用户。
#[cfg(debug_assertions)]
fn prune_dev_autostart<R: Runtime>(app: &AppHandle<R>) {
    if app.autolaunch().is_enabled().unwrap_or(false) {
        let _ = app.autolaunch().disable();
        let (message, tone) = choose_locale_message(
            &app.state::<AppState>().locale,
            "检测到开发模式的开机自启，已自动关闭：开发构建需配合开发服务器，系统启动时无服务器可连。",
            "Dev-mode autostart was auto-disabled: a dev build needs the dev server, which is absent at OS boot.",
        );
        emit_toast(app, &message, tone);
    }
}

/// 开发态下点「启用开机自启」被拒时的提示。
fn emit_autostart_blocked_toast<R: Runtime>(app: &AppHandle<R>) {
    let (message, tone) = choose_locale_message(
        &app.state::<AppState>().locale,
        "开发模式下开机自启无效（仅发布构建支持），已忽略。",
        "Launch at startup is unavailable in dev builds (release only); ignored.",
    );
    emit_toast(app, &message, tone);
}

/// 按缓存语言二选一（非法/缺省回落中文）。
fn choose_locale_message(
    locale: &std::sync::Mutex<String>,
    zh: &str,
    en: &str,
) -> (String, &'static str) {
    let is_en = locale
        .lock()
        .map(|g| g.starts_with("en"))
        .unwrap_or(false);
    if is_en {
        (en.to_string(), "warn")
    } else {
        (zh.to_string(), "warn")
    }
}

/// 向前端推送一条 toast（前端 `main.ts` 全局订阅 `app:toast` 事件后展示）。
fn emit_toast<R: Runtime>(app: &AppHandle<R>, message: &str, tone: &str) {
    let _ = app.emit(
        "app:toast",
        serde_json::json!({ "message": message, "tone": tone }),
    );
}

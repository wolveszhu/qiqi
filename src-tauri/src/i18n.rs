//! 轻量国际化：仅承载**系统托盘菜单**文案。
//!
//! 应用窗口 UI（设置层）的多语言由前端 `vue-i18n` 负责；Rust 侧只处理无 WebView 渲染、
//! 由操作系统直接绘制的托盘菜单。此处保持最小——仅中/英两套映射，按 locale 取用，
//! 非法/缺省回落 `zh-CN`（与前端 `i18n/index.ts` 的 `SUPPORTED_LOCALES` 对齐）。

/// 托盘菜单文案（集中常量，禁魔法字符串）。
pub struct TrayStrings {
    pub show: &'static str,
    pub hide: &'static str,
    pub settings: &'static str,
    /// 开机自启「关闭态」标签（当前未启用时显示，点击即启用）。
    pub autostart: &'static str,
    /// 开机自启「开启态」标签（当前已启用时显示，点击即关闭）。
    pub autostart_on: &'static str,
    pub quit: &'static str,
}

/// 受支持的语言白名单（与前端 `SUPPORTED_LOCALES` 保持一致；非法值回落默认）。
pub const SUPPORTED_LOCALES: &[&str] = &["zh-CN", "en-US"];

/// 按 locale 返回托盘菜单文案；非法/缺省回落 `zh-CN`。
///
/// @param locale - 语言标识（应为 `SUPPORTED_LOCALES` 之一）
/// @returns 对应语言的托盘菜单文案
pub fn tray_strings(locale: &str) -> TrayStrings {
    match locale {
        "en-US" => TrayStrings {
            show: "Show Pet",
            hide: "Hide Pet",
            settings: "Settings",
            autostart: "Launch at Startup",
            autostart_on: "Disable Launch at Startup",
            quit: "Quit",
        },
        _ => TrayStrings {
            show: "显示桌宠",
            hide: "隐藏桌宠",
            settings: "设置",
            autostart: "开机自启",
            autostart_on: "关闭开机自启",
            quit: "退出",
        },
    }
}

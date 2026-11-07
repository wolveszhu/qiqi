/**
 * 前端国际化中枢（vue-i18n，Composition API / legacy:false）。
 *
 * 仅覆盖**设置窗口（窗口 B）** UI 文案；桌宠窗口（窗口 A）固定文案与 LLM 提示词不在此范围。
 * 托盘菜单由 Rust 侧 `i18n` 模块负责。受支持语言与 Rust `i18n::SUPPORTED_LOCALES` 对齐。
 */

import { createI18n } from "vue-i18n";

import enUS from "./locales/en-US";
import zhCN from "./locales/zh-CN";

/** 受支持的语言标识。 */
export type Locale = "zh-CN" | "en-US";

/** 受支持语言白名单（与 Rust `i18n::SUPPORTED_LOCALES` 保持一致）。 */
export const SUPPORTED_LOCALES: readonly Locale[] = ["zh-CN", "en-US"] as const;

/** 默认/坏值回落语言。 */
export const DEFAULT_LOCALE: Locale = "zh-CN";

/**
 * 从 OS 语言推断默认语言（WebView2/Chromium 在 Windows 上反映 OS UI 语言）。
 *
 * @param osLang - `navigator.language` 返回值（如 "zh-CN" / "en-US"）
 * @returns 受支持的语言标识（中文系→zh-CN，其它→en-US）
 */
export function detectLocale(osLang: string): Locale {
  return osLang.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
}

const i18n = createI18n({
  legacy: false,
  locale: DEFAULT_LOCALE,
  fallbackLocale: DEFAULT_LOCALE,
  messages: { "zh-CN": zhCN, "en-US": enUS },
  // 开发期未翻译键不刷屏；缺省回落中文即可
  missingWarn: false,
  fallbackWarn: false,
});

export default i18n;

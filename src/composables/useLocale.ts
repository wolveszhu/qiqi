/**
 * 语言状态管理（模块级单例 ref，跨组件共享）。
 *
 * - `loadLocale()`：启动期调用，读后端持久化语言；首启为空则按 OS 语言检测并回写。
 * - `switchLocale()`：切换即改 `i18n.global.locale`（前端即时渲染）并落库（Rust 重建托盘）。
 *
 * 红线条：语言不进 localStorage / 日志；落库仅经 `api/locale` 单向到 Rust。
 */

import { ref, type Ref } from "vue";

import i18n, { DEFAULT_LOCALE, detectLocale, type Locale } from "../i18n";
import { getLocale, setLocale } from "../api/locale";

/** 当前语言（响应式，供 UI 绑定）。 */
const currentLocale = ref<Locale>(DEFAULT_LOCALE);

/** 防止重复 bootstrap（每个 webview 仅加载一次）。 */
let bootstrapped = false;

/** 同步应用语言到前端实例与响应式状态。 */
async function applyLocale(locale: Locale): Promise<void> {
  currentLocale.value = locale;
  i18n.global.locale.value = locale;
}

/**
 * 启动加载语言偏好（main.ts 在 mount 前 `await` 一次）。
 *
 * 后端 `get_locale` 用**空串**区分「未配置」与「显式选择中文」，避免每次启动
 * 都用 OS 语言覆盖用户选择。仅首启（空串）时按 OS 语言推断并回写；
 * 任何异常（非 Tauri 环境 / 读库失败）按 OS 语言回落，不阻断启动。
 */
export async function loadLocale(): Promise<void> {
  if (bootstrapped) return;
  bootstrapped = true;
  try {
    const stored = await getLocale();
    if (stored) {
      await applyLocale(stored === "en-US" ? "en-US" : "zh-CN");
      return;
    }
    // 首启未配置：按 OS 语言推断并持久化（同时让 Rust 重建托盘菜单）
    const detected = detectLocale(navigator.language);
    await applyLocale(detected);
    await setLocale(detected);
  } catch {
    await applyLocale(detectLocale(navigator.language));
  }
}

/**
 * 切换语言：先即时更新前端渲染，再落库（Rust 重建托盘）。
 * 落库失败回滚前端并向上抛出，由调用方 toast。
 *
 * @param locale - 目标语言
 */
export async function switchLocale(locale: Locale): Promise<void> {
  const previous = currentLocale.value;
  await applyLocale(locale);
  try {
    await setLocale(locale);
  } catch (e) {
    await applyLocale(previous); // 落库失败回滚前端状态
    throw e;
  }
}

/** 语言状态与操作方法。 */
export interface LocaleController {
  /** 当前语言（响应式）。 */
  currentLocale: Ref<Locale>;
  /** 启动加载语言偏好。 */
  loadLocale: () => Promise<void>;
  /** 切换并持久化语言。 */
  switchLocale: (locale: Locale) => Promise<void>;
}

/** 暴露语言状态与操作方法（供组件组合使用）。 */
export function useLocale(): LocaleController {
  return { currentLocale, loadLocale, switchLocale };
}

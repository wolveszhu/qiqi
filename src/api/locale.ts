/**
 * locale 命令封装（规范 §3：唯一 invoke 出口 = `callCommand`）。
 *
 * 语言偏好存于 SQLCipher `app_settings` type=5，前端不写 localStorage、不记日志。
 */

import { callCommand } from "./client";
import type { Locale } from "../i18n";

/** 读取已持久化的语言；**空串表示从未配置**（供首启 OS 语言检测）。 */
export function getLocale(): Promise<string> {
  return callCommand<string>("get_locale");
}

/** 写入语言并即时重建托盘菜单（Rust 侧落库 + 重建；非法值返回 Config 错误）。 */
export function setLocale(locale: Locale): Promise<void> {
  return callCommand("set_locale", { locale });
}

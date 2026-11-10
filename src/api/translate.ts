/**
 * 划词翻译命令封装（规范 §3：唯一 invoke 出口 = `callCommand`）。
 *
 * 红线：本模块不缓存、不落盘任何明文 Key；调用方必须 try/catch 并给用户可见反馈，
 * 错误由 `callCommand` 规整为 `AppError`（kind + message）后上抛。
 */

import { callCommand } from "./client";
import type {
  BuiltinTranslateProvider,
  CapturedSelection,
  CustomTranslateProvider,
  SaveCustomTranslateProvider,
  SaveTranslateSettings,
  TranslateResult,
  TranslateSettings,
} from "./types";

/** 读取配置（Key 脱敏回传）。 */
export function getTranslateSettings(): Promise<TranslateSettings> {
  return callCommand<TranslateSettings>("translate_settings_get");
}

/** 保存配置（Key 缺省不改动、空串清空）。 */
export async function saveTranslateSettings(
  payload: SaveTranslateSettings,
): Promise<void> {
  await callCommand("translate_settings_save", { payload });
}

/** 点击工具条后取词（手势阶段不调用）。 */
export function captureSelection(selectionKey: string): Promise<CapturedSelection> {
  return callCommand<CapturedSelection>("translate_capture_selection", { selectionKey });
}

/** 翻译文本；targetLang 缺省时按设置里的译入语言。 */
export function executeTranslate(
  text: string,
  targetLang?: string,
): Promise<TranslateResult> {
  return callCommand<TranslateResult>("translate_execute", {
    text,
    targetLang: targetLang ?? null,
  });
}

/** 重新翻译（同原文 + 当前服务）。 */
export function retryTranslate(resultId: string): Promise<TranslateResult> {
  return callCommand<TranslateResult>("translate_retry", { resultId });
}

/** 复制译文到系统剪贴板（用户主动点击）。 */
export async function copyTranslateResult(text: string): Promise<void> {
  await callCommand("translate_copy_result", { text });
}

/** 显示译文浮窗（Rust 侧定位到选区旁；compact=true 表示当前展示胶囊工具条）。 */
export async function showTranslatePopup(
  x: number,
  y: number,
  compact: boolean,
): Promise<void> {
  await callCommand("translate_popup_show", { x, y, compact });
}

/** 隐藏浮窗（窗口不销毁）。 */
export async function hideTranslatePopup(): Promise<void> {
  await callCommand("translate_popup_hide");
}

/** 重定位浮窗。 */
export async function moveTranslatePopup(
  x: number,
  y: number,
  compact: boolean,
): Promise<void> {
  await callCommand("translate_popup_position", { x, y, compact });
}

/** 用户自填的多个翻译服务商（Key 脱敏回传）。 */
export function listCustomTranslateProviders(): Promise<CustomTranslateProvider[]> {
  return callCommand<CustomTranslateProvider[]>("translate_list_custom_providers");
}

/** 新增 / 编辑自填服务商，返回行 id。 */
export function saveCustomTranslateProvider(
  payload: SaveCustomTranslateProvider,
): Promise<number> {
  return callCommand<number>("translate_save_custom_provider", { payload });
}

/** 删除自填服务商。 */
export async function deleteCustomTranslateProvider(id: number): Promise<void> {
  await callCommand("translate_delete_custom_provider", { id });
}

/** 上移 / 下移自填服务商（决定优先级）。 */
export async function moveCustomTranslateProvider(
  id: number,
  up: boolean,
): Promise<void> {
  await callCommand("translate_move_custom_provider", { id, up });
}

/** 两家内置服务商（百度 / 讯飞）的当月用量，只读，供配置类错误诊断使用。 */
export function listBuiltinTranslateProviders(): Promise<BuiltinTranslateProvider[]> {
  return callCommand<BuiltinTranslateProvider[]>("translate_list_builtin_providers");
}

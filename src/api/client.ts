/**
 * 全项目唯一 invoke 出口（《项目目录结构规范》§3 红线）。
 *
 * 纪律：任何模块不得直接 import `invoke`；命令一律经 `callCommand` 调用，
 * 错误在此规整为 `AppError`（kind + message），禁止裸 catch 后吞掉。
 */

import { invoke } from "@tauri-apps/api/core";

import type { AppError, AppErrorKind } from "./types";

/** Rust AppError 序列化结构（接口契约 §1：serde tag/content，kind 为 camelCase 小写）。 */
const APP_ERROR_KINDS: readonly AppErrorKind[] = [
  "db",
  "llm",
  "window",
  "probe",
  "io",
  "config",
  "unknown",
];

/** 将未知异常规整为 AppError（非 Rust 来源的错误兜底为 Unknown）。 */
export function normalizeError(e: unknown): AppError {
  if (typeof e === "object" && e !== null && "kind" in e) {
    const { kind, message } = e as { kind: unknown; message: unknown };
    if (typeof kind === "string" && APP_ERROR_KINDS.includes(kind as AppErrorKind)) {
      return {
        kind: kind as AppErrorKind,
        message: typeof message === "string" ? message : String(message),
      };
    }
  }
  return { kind: "unknown", message: e instanceof Error ? e.message : String(e) };
}

/**
 * 统一的 IPC 调用入口。
 *
 * @param cmd - 命令名（snake_case，契约见 docs/design/接口契约.md §2）
 * @param args - 参数对象（camelCase，serde rename_all 对齐）
 */
export async function callCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw normalizeError(e);
  }
}

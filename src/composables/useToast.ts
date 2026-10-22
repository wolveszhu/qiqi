/**
 * 全局轻提示队列（设计方案 §5.0：AppToast 单例队列）。
 *
 * 轻量、零业务：仅承载展示型提示文本与语义色。各面板调用 `pushToast(msg, tone)`
 * 反馈"已保存/失败"，不打断表单。2.4s 自动出队。
 */

import { reactive } from "vue";

export type ToastTone = "ok" | "error";

export interface ToastItem {
  id: number;
  tone: ToastTone;
  text: string;
}

const queue = reactive<ToastItem[]>([]);
let seq = 0;

/** 入队一条提示（2.4s 后自动消失）。 */
export function pushToast(text: string, tone: ToastTone = "ok"): void {
  const id = ++seq;
  queue.push({ id, tone, text });
  window.setTimeout((): void => {
    const i = queue.findIndex((t: ToastItem): boolean => t.id === id);
    if (i >= 0) queue.splice(i, 1);
  }, 2400);
}

/** 暴露响应式队列供 AppToast 渲染。 */
export function useToastQueue(): ToastItem[] {
  return queue;
}

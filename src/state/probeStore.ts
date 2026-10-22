/**
 * 探针快照镜像（规范 §3：事件驱动只读缓存；C6.2）。
 *
 * cursor 坐标 100ms 高频更新，**不走 reactive**（lookAt 直驱渲染器，性能红线）；
 * idle/foreground 低频翻转，进 reactive 供微随机引擎读取。
 */

import { reactive } from "vue";

export interface ProbeState {
  /** 是否处于空闲（≥ IDLE_THRESHOLD_MS，Rust 翻转事件驱动）。 */
  idle: boolean;
  /** 最近一次上报的空闲时长（毫秒）。 */
  idleMs: number;
  /** 前台进程名（仅进程名；null 表示无前台/桌面）。 */
  foreground: string | null;
}

const state = reactive<ProbeState>({
  idle: false,
  idleMs: 0,
  foreground: null,
});

/** idle 翻转写入（useSystemProbe 消费事件时调用）。 */
export function applyIdle(idle: boolean, idleMs: number): void {
  state.idle = idle;
  state.idleMs = idleMs;
}

/** 前台进程写入（useSystemProbe 消费事件时调用）。 */
export function applyForeground(process: string | null): void {
  state.foreground = process;
}

export const probeStore = state;

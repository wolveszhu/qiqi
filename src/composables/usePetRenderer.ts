/**
 * 渲染器生命周期（规范 §3）：PIXI 实例由**普通变量**持有，绝不进响应式。
 *
 * C2 接线：挂载后按 500ms 低频比对 `getHitRegion()`，变化才 invoke
 * `window_set_hit_region`（Rust 穿透状态机的输入；省 IPC）。
 * TODO(C6): probe:idle:changed → setIdleMode / playAction("sleep" | "stretch")。
 */

import { onMounted, onUnmounted } from "vue";

import { setHitRegion } from "../api/window";
import { createRenderer } from "../renderer/factory";
import type { IPetRenderer } from "../renderer/IPetRenderer";
import { petStore } from "../state/petStore";

/** hitRegion 上报间隔（C2.2：低频比对，变化才 invoke）。 */
const HIT_REGION_INTERVAL_MS = 500;

/** 组件内单例引用（跨 composable 调用共享同一实例）。 */
let renderer: IPetRenderer | null = null;

/** hitRegion 上报定时器（卸载时清理）。 */
let hitRegionTimer: ReturnType<typeof setInterval> | null = null;

/** usePetRenderer 暴露的渲染驱动 API（未挂载时 no-op）。 */
export interface UsePetRendererApi {
  playAction: (name: Parameters<IPetRenderer["playAction"]>[0]) => void;
  setExpression: (name: Parameters<IPetRenderer["setExpression"]>[0]) => void;
  lookAt: (x: number, y: number) => void;
}

/** 模块级驱动函数（操作单例 renderer；供其他 composable/组件直接调用，未挂载时 no-op）。 */
export function playAction(name: Parameters<IPetRenderer["playAction"]>[0]): void {
  renderer?.playAction(name);
}

export function setExpression(name: Parameters<IPetRenderer["setExpression"]>[0]): void {
  renderer?.setExpression(name);
}

/** 空闲降帧开关（C6.5 入睡/唤醒时联动，勘误#18 待机 30fps）。 */
export function setIdleMode(enabled: boolean): void {
  renderer?.setIdleMode?.(enabled);
}

export function lookAt(x: number, y: number): void {
  renderer?.lookAt(x, y);
}

export function usePetRenderer(): UsePetRendererApi {
  onMounted(async () => {
    const container = document.getElementById("pet-canvas");
    if (!container) return;
    renderer = createRenderer(petStore.activeRenderer ?? "code_cat");
    await renderer.mount(container);
    startHitRegionReporting();
  });

  onUnmounted((): void => {
    stopHitRegionReporting();
    renderer?.dispose();
    renderer = null;
  });

  return {
    /** 供其他 composable/组件驱动渲染器（未挂载时 no-op）。 */
    playAction: (name) => renderer?.playAction(name),
    setExpression: (name) => renderer?.setExpression(name),
    lookAt: (x, y) => renderer?.lookAt(x, y),
  };
}

/** 启动 hitRegion 低频上报（变化才 invoke；失败仅 warn——穿透回退默认开启）。 */
function startHitRegionReporting(): void {
  if (hitRegionTimer) return;
  let last = "";
  hitRegionTimer = setInterval((): void => {
    const current = renderer?.getHitRegion();
    if (!current) return;
    const key = `${current.x},${current.y},${current.width},${current.height}`;
    if (key === last) return; // 无变化：零 IPC
    last = key;
    void setHitRegion(current).catch((e: unknown) => {
      console.warn("[window] hitRegion 上报失败（穿透回退默认开启）", e);
    });
  }, HIT_REGION_INTERVAL_MS);
}

/** 停止上报（卸载时调用，防定时器泄漏）。 */
function stopHitRegionReporting(): void {
  if (hitRegionTimer) {
    clearInterval(hitRegionTimer);
    hitRegionTimer = null;
  }
}

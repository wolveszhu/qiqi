/**
 * 渲染器生命周期（规范 §3）：PIXI 实例由**普通变量**持有，绝不进响应式。
 *
 * C2 接线：挂载后按 500ms 低频比对 `getHitRegion()`，变化才 invoke
 * `window_set_hit_region`（Rust 穿透状态机的输入；省 IPC）。
 * E2.3①：待机降帧由 PetWindow 以 `probeIdle || sleepSnap` 单一真源调 `setIdleMode` 驱动 maxFPS。
 */

import { onMounted, onUnmounted, reactive } from "vue";

import { setHitRegion } from "../api/window";
import { createRenderer } from "../renderer/factory";
import type { IPetRenderer } from "../renderer/IPetRenderer";
import { SNAP_SHOW_RATIO_DEFAULT, type SnapEdge } from "../utils/edgeSnap";
import type { HitRegion } from "../api/types";
import { petStore } from "../state/petStore";

/** hitRegion 上报间隔（C2.2：低频比对，变化才 invoke）。 */
const HIT_REGION_INTERVAL_MS = 500;

/** 组件内单例引用（跨 composable 调用共享同一实例）。 */
let renderer: IPetRenderer | null = null;

/** hitRegion 上报定时器（卸载时清理）。 */
let hitRegionTimer: ReturnType<typeof setInterval> | null = null;
/** 额外命中矩形（临时 UI 需可点时写入，卸载清空）。 */
let extraHitRegion: HitRegion | null = null;
/** 上次上报指纹（定时器与 resize 立即上报共用，变化才 invoke）。 */
let lastRegionKey = "";

/** 额外矩形 union 进 hitRegion 上报（零位图，DOM 矩形）。 */
export function setExtraHitRegion(r: HitRegion | null): void {
  extraHitRegion = r;
}

/** 矩形并集（包围盒）；额外 UI 矩形 union 进角色命中区。 */
function unionRect(a: HitRegion, b: HitRegion): HitRegion {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  const r = Math.max(a.x + a.width, b.x + b.width);
  const btm = Math.max(a.y + a.height, b.y + b.height);
  return { x, y, width: r - x, height: btm - y };
}

/** 猫身命中区域（窗口 A 逻辑像素；气泡翻转决策的数据源，D1.1）。 */
export const hitRegion = reactive<HitRegion>({ x: 100, y: 100, width: 200, height: 200 });

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

/**
 * 边缘吸附休眠视觉（D4.4）：形象整体旋转 90° 躺下（左=顺时针 / 右=逆时针），
 * 并按 `showRatio` 决定露出圆盘的比例；唤醒恢复原状。
 */
export function setEdgeSleep(
  sleeping: boolean,
  side: SnapEdge | null = null,
  showRatio: number = SNAP_SHOW_RATIO_DEFAULT,
): void {
  renderer?.setEdgeSleep?.(sleeping, side, showRatio);
}

export function lookAt(x: number, y: number): void {
  renderer?.lookAt(x, y);
}

export function usePetRenderer(): UsePetRendererApi {
  onMounted(async () => {
    const container = document.getElementById("pet-canvas");
    if (!container) return;
    renderer = createRenderer(petStore.activeRenderer ?? "code_cat");
    try {
      await renderer.mount(container);
    } catch (e: unknown) {
      // 挂载失败不静默：角色不可见 + hitRegion 不上报（穿透回退默认开启，可诊断）
      console.error("[pet] 渲染器挂载失败，角色不可见", e);
      renderer = null;
      return;
    }
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

/** 比对并上报命中区（变化才 invoke；失败仅 warn——穿透回退默认开启）。 */
function reportHitRegion(): void {
  const current = renderer?.getHitRegion();
  if (!current) return;
  // 合并额外矩形（临时 UI 矩形入 hitRegion，穿透状态机照常）
  const region = extraHitRegion ? unionRect(current, extraHitRegion) : current;
  const key = `${region.x},${region.y},${region.width},${region.height}`;
  if (key === lastRegionKey) return;
  hitRegion.x = region.x;
  hitRegion.y = region.y;
  hitRegion.width = region.width;
  hitRegion.height = region.height;
  lastRegionKey = key;
  void setHitRegion(region).catch((e: unknown) => {
    console.warn("[window] hitRegion 上报失败（穿透回退默认开启）", e);
  });
}

/** 窗口尺寸变化 → 延后一帧重报（等渲染器 relayout 完成居中，防穿透误判）。 */
function onWindowResize(): void {
  requestAnimationFrame(reportHitRegion);
}

/** 启动 hitRegion 低频上报（变化才 invoke）。 */
function startHitRegionReporting(): void {
  if (hitRegionTimer) return;
  hitRegionTimer = setInterval(reportHitRegion, HIT_REGION_INTERVAL_MS);
  // 窗口 A 展开/收起：形象居中位置变化，立即重报一次（不等 500ms）
  window.addEventListener("resize", onWindowResize);
}

/** 停止上报（卸载时调用，防定时器与监听泄漏）。 */
function stopHitRegionReporting(): void {
  if (hitRegionTimer) {
    clearInterval(hitRegionTimer);
    hitRegionTimer = null;
  }
  window.removeEventListener("resize", onWindowResize);
}

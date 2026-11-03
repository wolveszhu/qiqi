/**
 * 渲染器生命周期（规范 §3）：PIXI 实例由**普通变量**持有，绝不进响应式。
 *
 * C2 接线：挂载后按 500ms 低频比对 `getHitRegion()`，变化才 invoke
 * `window_set_hit_region`（Rust 穿透状态机的输入；省 IPC）。
 * E2.3①：待机降帧由 PetWindow 以 `probeIdle || sleepSnap` 单一真源调 `setIdleMode` 驱动 maxFPS。
 */

import { onMounted, onUnmounted, reactive, watch } from "vue";

import { getState } from "../api/pet";
import { setHitRegion } from "../api/window";
import { createRenderer } from "../renderer/factory";
import type { IPetRenderer } from "../renderer/IPetRenderer";
import { SNAP_SHOW_RATIO_DEFAULT, type SnapEdge } from "../utils/edgeSnap";
import type { HitRegion } from "../api/types";
import { applyPetState, petStore } from "../state/petStore";

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

/** 最近一次表情（普通变量，非响应式）：形象重建后重放，避免表情闪回 neutral。 */
let lastExpression: Parameters<IPetRenderer["setExpression"]>[0] | null = null;

/** 当前渲染器对应的形象 id（用于幂等判定：同 id 不重建）。 */
let currentId: string | null = null;

/** 重建代号（只让最后一次重建生效，防止快速连切时两个 canvas 叠加）。 */
let rebuildGeneration = 0;

/** 重建完成后的回调（PetWindow 注册：重放休眠姿态 / 降帧等 UI 态）。 */
let afterRebuild: (() => void) | null = null;

/** 注册"重建完成"回调（卸载时应传 null 清空）。 */
export function onRendererRebuilt(fn: (() => void) | null): void {
  afterRebuild = fn;
}

/**
 * 销毁当前渲染器并按 id 重建（形象切换用）。
 *
 * 两条纪律：
 * - **幂等**：`id` 与当前一致直接返回——启动时 `applyPetState` 与 `pet:renderer:changed`
 *   事件都会改 store，重复重建只会闪一下；
 * - **只让最后一次生效**：清空容器 + 代号比对，避免快速连切时旧的 `mount()` 后完成、
 *   把 canvas 叠在新形象上（预览舞台曾因此同时画出猫和狗）。
 *
 * 重建期间需停 hitRegion 上报并**清零 `lastRegionKey`**——否则残留旧矩形的指纹会命中
 * "变化才 invoke"的短路判定，导致新形象命中区迟迟不上报、穿透误判最久 500ms。
 */
export async function rebuildRenderer(id: string): Promise<void> {
  if (!id || id === currentId) return;
  const container = document.getElementById("pet-canvas");
  if (!container) return;

  const gen = ++rebuildGeneration;
  currentId = id;

  stopHitRegionReporting();
  renderer?.dispose();
  renderer = null;
  lastRegionKey = "";
  // 兜底清空：`destroy()` 不保证把 canvas 从 DOM 摘掉
  container.innerHTML = "";

  const next = createRenderer(id);
  renderer = next;
  try {
    await next.mount(container);
  } catch (e: unknown) {
    console.error("[pet] 切换形象后渲染器挂载失败，角色不可见", e);
    renderer = null;
    currentId = null; // 允许同 id 重试（否则幂等判定会把重试挡掉）
    return;
  }
  // 期间又切了一次 → 本次结果作废（新人已在跑，勿夺权）
  if (gen !== rebuildGeneration) return;

  if (lastExpression) next.setExpression(lastExpression);
  startHitRegionReporting();
  reportHitRegion(); // 立即补报一次，不等 500ms
  afterRebuild?.();
}

export function usePetRenderer(): UsePetRendererApi {
  onMounted(async () => {
    const container = document.getElementById("pet-canvas");
    if (!container) return;
    // 先拉回**持久化的**形象 id 再建渲染器：store 初始值是 `code_cat`，
    // 若直接建，用户存的是狗也会先画猫、再靠下面的 watch 跳变（重启后表现为"显示成猫"）。
    try {
      applyPetState(await getState());
    } catch (e: unknown) {
      console.warn("[pet] 启动读取桌宠状态失败（沿用默认形象）", e);
    }
    currentId = petStore.activeRenderer ?? "code_cat";
    renderer = createRenderer(currentId);
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

  /**
   * 形象 id 变化 → 重建。
   *
   * **启动链路的关键**：窗口 A 挂载时先按 store 的初始值 `code_cat` 建渲染器，
   * 之后 `useProactive` / `useAffection` 才异步 `getState()` 把真实 id
   * （如 `code_dog`）写进 store——若不在此重建，重启后永远显示默认猫。
   */
  watch(
    () => petStore.activeRenderer,
    (id): void => {
      if (id) void rebuildRenderer(id);
    },
  );

  onUnmounted((): void => {
    stopHitRegionReporting();
    renderer?.dispose();
    renderer = null;
    currentId = null;
    afterRebuild = null;
  });

  return {
    /** 供其他 composable/组件驱动渲染器（未挂载时 no-op）。 */
    playAction: (name) => renderer?.playAction(name),
    setExpression: (name) => {
      lastExpression = name;
      renderer?.setExpression(name);
    },
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

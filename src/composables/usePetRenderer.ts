/**
 * 渲染器生命周期（规范 §3）：PIXI 实例由**普通变量**持有，绝不进响应式。
 *
 * TODO(C1): mount 后启动呼吸 Ticker；TODO(C2): 将 getHitRegion 上报 window_set_hit_region；
 * TODO(C6): probe:idle:changed → setIdleMode/playAction("sleep"|"stretch")。
 */

import { onMounted, onUnmounted } from "vue";

import { createRenderer } from "../renderer/factory";
import type { IPetRenderer } from "../renderer/IPetRenderer";
import { petStore } from "../state/petStore";

/** 组件内单例引用（跨 composable 调用共享同一实例）。 */
let renderer: IPetRenderer | null = null;

/** usePetRenderer 暴露的渲染驱动 API（未挂载时 no-op）。 */
export interface UsePetRendererApi {
  playAction: (name: Parameters<IPetRenderer["playAction"]>[0]) => void;
  setExpression: (name: Parameters<IPetRenderer["setExpression"]>[0]) => void;
  lookAt: (x: number, y: number) => void;
}

export function usePetRenderer(): UsePetRendererApi {
  onMounted(async () => {
    const container = document.getElementById("pet-canvas");
    if (!container) return;
    renderer = createRenderer(petStore.activeRenderer ?? "code_cat");
    await renderer.mount(container);
  });

  onUnmounted((): void => {
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

<script setup lang="ts">
/**
 * 窗口 A（桌宠层 400×400 透明）：PIXI 画布挂载 + 点击果冻反馈。
 *
 * C2 桌面融合接线：
 * - pointerdown → `window_start_drag`（系统接管拖拽，前端只做形变反馈）；
 * - hitRegion 低频上报由 `usePetRenderer` 内部完成（源头持有者）；
 * - cursor 事件 → 渲染器 lookAt（真实数据源，取代 C1 开发期 mousemove）。
 *
 * TODO(D1): SpeechBubble/ChatInput 接入 chatStore；TODO(D4): BellToggle 头顶铃铛。
 * 纪律：PIXI 实例由 `usePetRenderer` 以普通变量持有，不进响应式（code-standards §6）。
 */

import { startDrag } from "../../api/window";
import { usePetRenderer } from "../../composables/usePetRenderer";
import { useSystemProbe } from "../../composables/useSystemProbe";

const { playAction, lookAt } = usePetRenderer();
useSystemProbe((x, y) => lookAt(x, y));

/**
 * 按下即拖拽：透明区已被 Rust 穿透（点击不到本 DOM），
 * 因此 pointerdown 到达此处即视为命中猫身（C2.5 链路）。
 */
function onPointerDown(): void {
  playAction("drag");
  void startDrag().catch((e: unknown) => {
    console.warn("[window] 启动拖拽失败", e);
  });
}
</script>

<template>
  <div class="pet-window">
    <!-- PIXI 画布容器（透明窗口；穿透由 C2 Rust 状态机接管） -->
    <div
      id="pet-canvas"
      class="pet-window__canvas"
      @pointerdown="onPointerDown"
    ></div>
    <p class="pet-window__hint">七七（C2 桌面融合；D1/D4 落位气泡与铃铛）</p>
  </div>
</template>

<style scoped>
.pet-window {
  width: 100vw;
  height: 100vh;
}

/* 画布容器：400×400 逻辑像素居中（与窗口 A 尺寸一致） */
.pet-window__canvas {
  width: 400px;
  height: 400px;
  margin: 0 auto;
}

.pet-window__hint {
  position: absolute;
  bottom: 6px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 11px;
  color: var(--ink-soft);
  pointer-events: none;
}
</style>

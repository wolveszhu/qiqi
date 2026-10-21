<script setup lang="ts">
/**
 * 窗口 A（桌宠层 400×400 透明）：PIXI 画布挂载 + 点击果冻反馈。
 *
 * TODO(D1): SpeechBubble/ChatInput 接入 chatStore；TODO(D4): BellToggle 头顶铃铛。
 * 注意：canvas 容器尺寸与窗口 A 的 400×400 一致（B1 已配置）；PIXI 实例由
 * `usePetRenderer` 以普通变量持有，不进响应式（code-standards §6）。
 */

import { usePetRenderer } from "../../composables/usePetRenderer";

const { playAction } = usePetRenderer();
</script>

<template>
  <div class="pet-window">
    <!-- PIXI 画布容器（透明窗口；穿透由 C2 接管） -->
    <div
      id="pet-canvas"
      class="pet-window__canvas"
      @pointerdown="playAction('click')"
    >
      <p class="pet-window__hint">七七（C1 渲染内核；D1/D4 落位气泡与铃铛）</p>
    </div>
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

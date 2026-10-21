<script setup lang="ts">
/**
 * 窗口 A（桌宠层，400×400 透明）：渲染器挂载 + 气泡占位 + 铃铛预留。
 *
 * TODO(C1): #pet-canvas 挂载后 usePetRenderer.mount；
 * TODO(D1): SpeechBubble/ChatInput 接入 chatStore；TODO(D4): BellToggle 头顶铃铛。
 */

import { usePetRenderer } from "../../composables/usePetRenderer";

usePetRenderer();
</script>

<template>
  <div class="pet-window">
    <!-- PIXI 画布容器（透明窗口，点击穿透由 C2 接管） -->
    <div
      id="pet-canvas"
      class="pet-window__canvas"
    ></div>

    <!-- TODO(D1): SpeechBubble（chatStore.bubbleText）+ ChatInput（Ctrl+Shift+P） -->
    <!-- TODO(D4): BellToggle（头顶铃铛） -->
    <p class="pet-window__placeholder">七七骨架（C1/D1/D4 落位）</p>
  </div>
</template>

<style scoped>
.pet-window {
  position: relative;
  width: 100vw;
  height: 100vh;
}

.pet-window__canvas {
  position: absolute;
  inset: 0;
}

.pet-window__placeholder {
  position: absolute;
  bottom: 8px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 11px;
  color: var(--ink-soft);
}
</style>

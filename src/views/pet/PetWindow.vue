<script setup lang="ts">
/**
 * 窗口 A（桌宠层 400×400 透明）：PIXI 画布挂载 + 点击果冻反馈。
 *
 * C2/C4 接线：
 * - pointerdown → `window_start_drag`（系统接管拖拽，前端只做形变反馈）；
 * - hitRegion 低频上报由 `usePetRenderer` 内部完成（源头持有者）；
 * - cursor 事件 → 渲染器 lookAt（真实数据源）；
 * - 聊天流 → chatStore → 气泡占位（D1 换正式 SpeechBubble 组件）。
 *
 * TODO(D1): SpeechBubble/ChatInput 正式组件；TODO(D4): BellToggle 头顶铃铛。
 * 纪律：PIXI 实例由 `usePetRenderer` 以普通变量持有，不进响应式（code-standards §6）。
 */

import { startDrag } from "../../api/window";
import { useChatStream } from "../../composables/useChatStream";
import { usePetRenderer } from "../../composables/usePetRenderer";
import { useSystemProbe } from "../../composables/useSystemProbe";
import { chatStore } from "../../state/chatStore";

const { playAction, lookAt } = usePetRenderer();
useSystemProbe((x, y) => lookAt(x, y));
// 仅挂载事件订阅（send/abortCurrent 由 D1 的 ChatInput 组件消费）
useChatStream();

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
    <!-- 气泡占位（D1 换正式 SpeechBubble；单气泡覆盖式，勘误#12） -->
    <div
      v-if="chatStore.text || chatStore.isStreaming"
      class="pet-window__bubble"
    >
      {{ chatStore.text
      }}<span
        v-if="chatStore.isStreaming"
        class="pet-window__cursor"
        >▍</span
      >
    </div>

    <!-- PIXI 画布容器（透明窗口；穿透由 C2 Rust 状态机接管） -->
    <div
      id="pet-canvas"
      class="pet-window__canvas"
      @pointerdown="onPointerDown"
    ></div>
    <p class="pet-window__hint">七七（C4 管道就绪；D1 落正式气泡与输入框）</p>
  </div>
</template>

<style scoped>
.pet-window {
  width: 100vw;
  height: 100vh;
}

/* 气泡占位：顶部居中（D1 按设计方案样式替换） */
.pet-window__bubble {
  position: absolute;
  top: 8px;
  left: 50%;
  transform: translateX(-50%);
  max-width: 90%;
  padding: 8px 12px;
  border-radius: 12px;
  background: var(--bg-window, #fffef9);
  color: var(--ink, #403a2e);
  font-size: 13px;
  line-height: 1.5;
  box-shadow: 0 2px 8px rgb(0 0 0 / 12%);
  pointer-events: none;
  word-break: break-word;
}

.pet-window__cursor {
  animation: blink 1s steps(2) infinite;
}

@keyframes blink {
  50% {
    opacity: 0;
  }
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
  color: var(--ink-soft, #8a8272);
  pointer-events: none;
}
</style>

<script setup lang="ts">
import { computed } from "vue";

import { hitRegion } from "../../composables/usePetRenderer";
import { resolveBubbleLayout } from "../../utils/bubbleLayout";

/** 窗口 A 逻辑宽度（与设计 token WINDOW_SIZE 一致）。 */
const WINDOW_W = 400;
const MODEL_MAX = 300;
const MODEL_MIN = 120;
const CHAR_W = 7;

const props = withDefaults(
  defineProps<{
    /** 气泡文本。 */
    text?: string;
    /** 是否处于流式（显示光标）。 */
    streaming?: boolean;
    /** 是否可见（空文本或 false → 不渲染）。 */
    visible?: boolean;
  }>(),
  { text: "", streaming: false, visible: true },
);

// 估算气泡期望宽度（防截断：真实宽度由文本长度估算，布局再按空间 clamp）
const desiredW = computed(() => {
  const chars = [...props.text].length;
  return Math.min(MODEL_MAX, Math.max(MODEL_MIN, chars * CHAR_W + 28));
});

const layout = computed(() =>
  resolveBubbleLayout({
    winW: WINDOW_W,
    catRegion: hitRegion,
    bubbleW: desiredW.value,
  }),
);

const style = computed(() => ({
  left: `${layout.value.x}px`,
  maxWidth: `${layout.value.maxWidth}px`,
}));
</script>

<template>
  <transition name="bubble">
    <div
      v-if="visible && text"
      class="bubble"
      :class="layout.side"
      :style="style"
      role="status"
      aria-live="polite"
    >
      <div
        class="bubble-text"
        :class="{ streaming }"
      >
        {{ text
        }}<span
          v-if="streaming"
          class="caret"
          >▍</span
        >
      </div>
      <span class="tail"></span>
    </div>
  </transition>
</template>

<style scoped>
.bubble {
  position: absolute;
  bottom: 58%;
  z-index: 10;
  min-width: 64px;
  padding: 8px 12px;
  border-radius: 14px;
  background: var(--surface-bubble);
  color: var(--text-primary);
  box-shadow: 0 4px 14px var(--shadow-soft);
  font: 500 13px/1.45 var(--font-ui);
  word-break: break-word;
  white-space: pre-wrap;
}
.bubble-text.streaming .caret {
  margin-left: 1px;
  animation: caret-blink 1s steps(1) infinite;
}
@keyframes caret-blink {
  50% {
    opacity: 0;
  }
}
/* 指猫小三角：右侧气泡→左下，左侧气泡→右下 */
.tail {
  position: absolute;
  bottom: -6px;
  width: 12px;
  height: 12px;
  background: var(--surface-bubble);
  transform: rotate(45deg);
  border-radius: 2px;
}
.bubble.right .tail {
  left: 16px;
}
.bubble.left .tail {
  right: 16px;
}
/* 进入/离开过渡（轻量，不影响逐字打字机） */
.bubble-enter-active,
.bubble-leave-active {
  transition: opacity var(--dur-fast) var(--ease-out);
}
.bubble-enter-from,
.bubble-leave-to {
  opacity: 0;
}
</style>

<script setup lang="ts">
/**
 * 全局轻提示（设计方案 §5.0）：右下角浮出，白底卡片风，成功绿点/失败红点。
 * 消费 `useToast` 单例队列（纯展示，零业务依赖）。
 */
import { useToastQueue } from "../../composables/useToast";

const queue = useToastQueue();
</script>

<template>
  <div class="toast-layer">
    <transition-group name="toast">
      <div
        v-for="t in queue"
        :key="t.id"
        class="toast"
        :class="`toast--${t.tone}`"
        role="status"
      >
        <span class="toast__dot"></span>
        <span class="toast__text">{{ t.text }}</span>
      </div>
    </transition-group>
  </div>
</template>

<style scoped>
.toast-layer {
  position: fixed;
  right: 20px;
  bottom: 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 100;
  pointer-events: none;
}
.toast {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: var(--bg-window);
  border: 1px solid var(--line);
  border-radius: 12px;
  box-shadow: var(--shadow-card);
  font-size: 13px;
  color: var(--ink);
}
.toast__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex: none;
}
.toast--ok .toast__dot {
  background: var(--matcha);
}
.toast--error .toast__dot {
  background: var(--peach);
}
.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>

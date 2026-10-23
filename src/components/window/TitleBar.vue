<script setup lang="ts">
/**
 * 自绘标题栏（设计方案 §3 / B1）：拖拽区 + 关闭按钮（B 关闭=hide，C2 托盘提供退出）。
 * TODO(B1): data-tauri-drag-region 验证；TODO(C2): ✕ 改调 window_hide（替换临时 emit）。
 */

import { ref } from "vue";

const emit = defineEmits<{ (e: "close"): void }>();

const kbdHint = ref("Ctrl+Shift+P");
</script>

<template>
  <header
    class="titlebar"
    data-tauri-drag-region
  >
    <img
      class="titlebar__logo"
      src="/logo.png"
      alt="Qiqi"
      draggable="false"
    />
    <span class="titlebar__kbd">{{ kbdHint }}</span>
    <button
      class="titlebar__close"
      aria-label="关闭"
      @click="emit('close')"
    >
      ✕
    </button>
  </header>
</template>

<style scoped>
.titlebar {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 48px;
  padding: 0 16px;
  background: var(--bg-window);
  border-bottom: 1px solid var(--line);
}

.titlebar__logo {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  user-select: none;
  -webkit-user-drag: none;
}

.titlebar__kbd {
  font-size: 12px;
  color: var(--ink-soft);
}

.titlebar__close {
  margin-left: auto;
  border: none;
  background: none;
  font-size: 14px;
  color: var(--ink-soft);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: var(--r-sm);
}

.titlebar__close:hover {
  background: var(--peach-soft);
  color: var(--peach);
}
</style>

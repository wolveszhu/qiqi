<script setup lang="ts">
/**
 * 自绘标题栏（设计方案 §3 / B1）：拖拽区 + 关闭按钮（B 关闭=hide，C2 托盘提供退出）。
 * TODO(B1): data-tauri-drag-region 验证；TODO(C2): ✕ 改调 window_hide（替换临时 emit）。
 */

import { ref, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

const { t } = useI18n();

const emit = defineEmits<{ (e: "close"): void }>();

const kbdHint = ref("七七");

// 窗口控制（窗口 B 自绘标题栏，无系统装饰）：最小化到任务栏 + 最大化/还原。
const appWindow = getCurrentWebviewWindow();
const isMax = ref(false);

async function syncMax() {
  isMax.value = await appWindow.isMaximized();
}
function minimize() {
  void appWindow.minimize();
}
function toggleMaximize() {
  void appWindow.toggleMaximize();
}

// 监听窗口尺寸变化同步最大化态（卸载时反注册，防泄漏）。
let unlisten: (() => void) | null = null;
onMounted(async () => {
  await syncMax();
  unlisten = await appWindow.onResized(() => void syncMax());
});
onUnmounted(() => unlisten?.());
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
      class="titlebar__btn"
      :aria-label="t('titleBar.minimize')"
      @click="minimize"
      @mousedown.stop.prevent
    >
      —
    </button>
    <button
      class="titlebar__btn"
      :aria-label="isMax ? t('titleBar.restore') : t('titleBar.maximize')"
      @click="toggleMaximize"
      @mousedown.stop.prevent
    >
      {{ isMax ? "❐" : "▢" }}
    </button>
    <button
      class="titlebar__close"
      :aria-label="t('titleBar.close')"
      @click="emit('close')"
      @mousedown.stop.prevent
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

.titlebar__btn {
  border: none;
  background: none;
  font-size: 14px;
  color: var(--ink-soft);
  cursor: pointer;
  padding: 4px 10px;
  border-radius: var(--r-sm);
}

.titlebar__btn:hover {
  background: rgba(0, 0, 0, 0.06);
  color: var(--ink);
}

.titlebar__btn:first-of-type {
  margin-left: auto;
}

.titlebar__close {
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

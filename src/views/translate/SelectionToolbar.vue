<script setup lang="ts">
/**
 * 选区旁悬浮小图标（元宝划词工具栏同款交互）：只展示，点击由父级取词。
 */

import { useI18n } from "vue-i18n";

withDefaults(defineProps<{ loading?: boolean }>(), { loading: false });

const emit = defineEmits<{ (e: "capture"): void }>();
const { t } = useI18n();
</script>

<template>
  <button
    class="toolbar"
    type="button"
    :aria-label="t('translate.toolbar.label')"
    :title="t('translate.toolbar.hint')"
    :disabled="loading"
    @click="emit('capture')"
  >
    <span class="toolbar__glyph">文</span>
    <span
      v-if="loading"
      class="toolbar__spinner"
      aria-hidden="true"
    ></span>
  </button>
</template>

<style scoped>
.toolbar {
  position: absolute;
  top: 8px;
  left: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border: 1px solid var(--matcha-deep);
  border-radius: 12px;
  background: var(--bg-window);
  box-shadow: var(--shadow-card);
  color: var(--ink);
  font-family: var(--font-disp);
  font-size: 15px;
  cursor: pointer;
  transition: transform var(--dur-fast) var(--ease-jelly);
}

.toolbar:hover:not(:disabled) {
  transform: translateY(-2px);
}

.toolbar:disabled {
  opacity: 0.6;
  cursor: wait;
}

.toolbar__spinner {
  position: absolute;
  right: 3px;
  bottom: 3px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--peach);
}
</style>

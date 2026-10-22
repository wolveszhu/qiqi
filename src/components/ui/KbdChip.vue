<script setup lang="ts">
/**
 * 快捷键胶囊（设计方案 §5.1）：只读展示（MVP 不支持改键）。
 * ui/ 红线：零业务依赖，仅展示传入字符串（如 "Ctrl+Shift+P"）。
 */
defineProps<{ keys: string }>();
</script>

<template>
  <span class="kbd">{{ keys }}</span>
</template>

<style scoped>
.kbd {
  display: inline-flex;
  align-items: center;
  padding: 2px 9px 3px;
  background: var(--bg-window);
  border: 1px solid var(--line);
  border-bottom-width: 3px;
  border-radius: 8px;
  font-family: Consolas, monospace;
  font-size: 11px;
  font-weight: 700;
  color: var(--ink);
  white-space: nowrap;
}
</style>

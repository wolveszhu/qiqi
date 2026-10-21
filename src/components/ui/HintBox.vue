<script setup lang="ts">
/**
 * 提示框（设计方案 §5.1）：引导/计费说明/必选约束提示的统一样式。
 */

withDefaults(defineProps<{ tone?: "info" | "warn" }>(), { tone: "info" });
</script>

<template>
  <aside
    class="hint-box"
    :class="`hint-box--${tone}`"
  >
    <slot></slot>
  </aside>
</template>

<style scoped>
.hint-box {
  padding: 10px 14px;
  border-radius: var(--r-sm);
  font-size: 13px;
  line-height: 1.6;
}

.hint-box--info {
  background: var(--matcha-soft);
  color: var(--matcha-deep);
}

.hint-box--warn {
  background: var(--butter);
  color: var(--ink);
}
</style>

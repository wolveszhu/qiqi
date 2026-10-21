<script setup lang="ts">
/**
 * 状态徽标（设计方案 §5.1）：抹茶/桃/灰三色语义。
 */

withDefaults(defineProps<{ tone?: "ok" | "warn" | "muted"; text: string }>(), {
  tone: "muted",
});
</script>

<template>
  <span
    class="status-badge"
    :class="`status-badge--${tone}`"
    >{{ text }}</span
  >
</template>

<style scoped>
.status-badge {
  display: inline-flex;
  align-items: center;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
}

.status-badge--ok {
  background: var(--matcha-soft);
  color: var(--matcha-deep);
}

.status-badge--warn {
  background: var(--peach-soft);
  color: var(--peach);
}

.status-badge--muted {
  background: var(--bg-deep);
  color: var(--ink-soft);
}
</style>

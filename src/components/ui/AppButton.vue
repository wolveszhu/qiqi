<script setup lang="ts">
/**
 * 通用按钮（设计方案 §5.1）：抹茶主色/次级/危险三变体 + 方形胶囊。
 * ui/ 红线：零业务依赖，仅 props/emits（规范 §3）。
 */

withDefaults(
  defineProps<{
    variant?: "primary" | "ghost" | "danger";
    disabled?: boolean;
    loading?: boolean;
  }>(),
  { variant: "primary", disabled: false, loading: false },
);

defineEmits<{ (e: "click", ev: MouseEvent): void }>();
</script>

<template>
  <button
    class="app-btn"
    :class="[`app-btn--${variant}`, { 'is-loading': loading }]"
    :disabled="disabled || loading"
    @click="$emit('click', $event)"
  >
    <span
      v-if="loading"
      class="app-btn__spin"
      aria-hidden="true"
    ></span>
    <slot></slot>
  </button>
</template>

<style scoped>
.app-btn {
  padding: 10px 22px;
  border: none;
  border-radius: var(--r-sm);
  font-family: var(--font-body);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: transform var(--dur-instant, 120ms) var(--ease-out, ease);
}

.app-btn:active {
  transform: scale(0.96);
}

.app-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.app-btn__spin {
  display: inline-block;
  width: 14px;
  height: 14px;
  margin-right: 6px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  vertical-align: -2px;
  animation: app-btn-spin 0.7s linear infinite;
}

@keyframes app-btn-spin {
  to {
    transform: rotate(360deg);
  }
}

.app-btn--primary {
  background: var(--matcha);
  color: var(--bg-window);
}

.app-btn--primary:hover {
  background: var(--matcha-deep);
}

.app-btn--ghost {
  background: var(--bg-soft);
  color: var(--ink);
  border: 1px solid var(--line);
}

.app-btn--danger {
  background: var(--peach);
  color: var(--bg-window);
}
</style>

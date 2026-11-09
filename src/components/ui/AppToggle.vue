<script setup lang="ts">
/**
 * 开关（设计方案 §5.1）：AppToggle.vue 弹簧曲线（--ease-spring 同族）。
 * 受控组件：v-model 语义（modelValue + update:modelValue）。
 */

defineProps<{ modelValue: boolean; label?: string; ariaLabel?: string }>();

const emit = defineEmits<{ (e: "update:modelValue", value: boolean): void }>();
</script>

<template>
  <label class="app-toggle">
    <span
      v-if="label"
      class="app-toggle__label"
      >{{ label }}</span
    >
    <button
      class="app-toggle__track"
      :class="{ 'is-on': modelValue }"
      type="button"
      role="switch"
      :aria-label="ariaLabel"
      :aria-checked="modelValue"
      @click="emit('update:modelValue', !modelValue)"
    >
      <span class="app-toggle__thumb"></span>
    </button>
  </label>
</template>

<style scoped>
.app-toggle {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
}

.app-toggle__label {
  font-size: 14px;
  color: var(--ink);
}

.app-toggle__track {
  width: 48px;
  height: 26px;
  border: none;
  border-radius: 999px;
  background: var(--line);
  padding: 3px;
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-spring);
}

.app-toggle__track.is-on {
  background: var(--matcha);
}

.app-toggle__thumb {
  display: block;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--bg-window);
  transition: transform var(--dur-fast) var(--ease-spring);
}

.app-toggle__track.is-on .app-toggle__thumb {
  transform: translateX(22px);
}
</style>

<script setup lang="ts">
/**
 * 输入框（设计方案 §5.1）：label + 可选密码显隐（D2 Key 输入）。
 * 受控组件：modelValue + update:modelValue。
 */

withDefaults(
  defineProps<{
    modelValue: string;
    label?: string;
    type?: "text" | "password";
    placeholder?: string;
    /** 密码显隐切换按钮（type=password 时默认开启）。 */
    revealable?: boolean;
    /** 最大长度（如桌宠名字 ≤20）。 */
    maxlength?: number;
  }>(),
  { type: "text", revealable: true },
);

const emit = defineEmits<{ (e: "update:modelValue", value: string): void }>();

import { ref } from "vue";

const revealed = ref(false);
</script>

<template>
  <label class="app-input">
    <span
      v-if="label"
      class="app-input__label"
      >{{ label }}</span
    >
    <span class="app-input__wrap">
      <input
        class="app-input__field"
        :type="type === 'password' && !revealed ? 'password' : 'text'"
        :placeholder="placeholder"
        :maxlength="maxlength"
        :value="modelValue"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      />
      <button
        v-if="type === 'password' && revealable"
        class="app-input__reveal"
        type="button"
        @click="revealed = !revealed"
      >
        {{ revealed ? "隐藏" : "显示" }}
      </button>
    </span>
  </label>
</template>

<style scoped>
.app-input {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.app-input__label {
  font-size: 13px;
  color: var(--ink-soft);
}

.app-input__wrap {
  position: relative;
  display: flex;
}

.app-input__field {
  flex: 1;
  padding: 9px 12px;
  border: 1px solid var(--line);
  border-radius: var(--r-sm);
  background: var(--bg-soft);
  font-size: 14px;
  color: var(--ink);
}

.app-input__field:focus {
  outline: 2px solid var(--matcha-soft);
  border-color: var(--matcha);
}

.app-input__reveal {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  border: none;
  background: none;
  color: var(--ink-soft);
  font-size: 12px;
  cursor: pointer;
}
</style>

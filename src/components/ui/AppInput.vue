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
    /** 禁用（只读展示；如预设平台名锁定不可改）。 */
    disabled?: boolean;
  }>(),
  { type: "text", revealable: true, disabled: false },
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
        :class="{ 'app-input__field--revealable': type === 'password' && revealable }"
        :type="type === 'password' && !revealed ? 'password' : 'text'"
        :placeholder="placeholder"
        :maxlength="maxlength"
        :disabled="disabled"
        :value="modelValue"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      />
      <button
        v-if="type === 'password' && revealable"
        class="app-input__reveal"
        type="button"
        :aria-label="revealed ? '隐藏 API Key' : '显示 API Key'"
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

/* 密码字段为右侧显隐按钮留位，避免长文本压在按钮下 */
.app-input__field--revealable {
  padding-right: 56px;
}

/*
 * WebView2/Edge 会给 password 输入框渲染**原生显隐"小眼睛"**（::-ms-reveal 伪元素），
 * 与本组件的「显示/隐藏」按钮重复且位置重叠——此处隐藏原生控件，只保留应用自己的
 * （跨浏览器一致：Chromium 系走 ::-ms-reveal，其他内核本就不会渲染）。
 */
.app-input__field::-ms-reveal {
  display: none;
}

.app-input__field:focus {
  outline: 2px solid var(--matcha-soft);
  border-color: var(--matcha);
}

.app-input__field:disabled {
  opacity: 0.65;
  cursor: not-allowed;
  color: var(--ink-soft);
}

/* 显隐按钮：小药丸样式，避免与输入框边框/聚焦环视觉冲突 */
.app-input__reveal {
  position: absolute;
  right: 6px;
  top: 50%;
  transform: translateY(-50%);
  border: 1px solid var(--line);
  border-radius: 999px;
  padding: 2px 9px;
  background: var(--bg-window);
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.4;
  cursor: pointer;
  transition:
    color var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out);
}

.app-input__reveal:hover {
  color: var(--matcha-deep);
  border-color: var(--matcha);
}
</style>

<script setup lang="ts">
/**
 * 对话输入 composer（窗口 B「对话」Tab 使用）。
 *
 * 原窗口 A 版本里的点击穿透切换（`setClickThroughEnabled`）与可见性 prop 属**桌宠层专属**，
 * 对话迁入窗口 B（普通窗口、无穿透）后已移除，本组件只保留纯输入职责。
 */

import { ref } from "vue";

withDefaults(
  defineProps<{
    /** 流式中禁用发送（防并发流）。 */
    disabled?: boolean;
  }>(),
  { disabled: false },
);

const emit = defineEmits<{ (e: "send", text: string): void }>();

const draft = ref("");

function submit(): void {
  const text = draft.value.trim();
  if (!text) return;
  emit("send", text);
  draft.value = "";
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    submit();
  }
}
</script>

<template>
  <div class="composer">
    <textarea
      v-model="draft"
      class="composer__field"
      rows="2"
      placeholder="说点什么…（Enter 发送 / Shift+Enter 换行）"
      :disabled="disabled"
      @keydown="onKeydown"
    ></textarea>
    <button
      class="composer__send"
      type="button"
      :disabled="disabled || draft.trim().length === 0"
      @click="submit"
    >
      发送
    </button>
  </div>
</template>

<style scoped>
.composer {
  display: flex;
  align-items: stretch;
  gap: 8px;
}
.composer__field {
  flex: 1;
  resize: none;
  border-radius: 12px;
  border: 1px solid var(--border-soft);
  background: var(--surface-panel);
  color: var(--text-primary);
  padding: 8px 10px;
  font: 500 13px/1.4 var(--font-ui);
  outline: none;
  transition: border-color var(--dur-fast) var(--ease-out);
}
.composer__field:focus {
  border-color: var(--accent-rose);
}
.composer__field:disabled {
  opacity: 0.6;
}
.composer__send {
  align-self: stretch;
  padding: 0 16px;
  border: none;
  border-radius: 12px;
  background: var(--accent-rose);
  color: #fff;
  font: 600 13px var(--font-ui);
  cursor: pointer;
  transition: filter var(--dur-fast) var(--ease-out);
}
.composer__send:hover:not(:disabled) {
  filter: brightness(1.05);
}
.composer__send:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>

<script setup lang="ts">
/**
 * 对话输入 composer（窗口 B「对话」Tab 使用）。
 *
 * 原窗口 A 版本里的点击穿透切换（`setClickThroughEnabled`）与可见性 prop 属**桌宠层专属**，
 * 对话迁入窗口 B（普通窗口、无穿透）后已移除，本组件只保留纯输入职责。
 *
 * 「联网搜索」开关：受控 `webSearch`（`v-model:webSearch`），开/关状态由父级持有，
 * 仅做展示与 emit，不耦合任何业务逻辑（搜索是否真正发起由后端按配置裁决）。
 */

import { ref } from "vue";

withDefaults(
  defineProps<{
    /** 流式中禁用发送（防并发流）。 */
    disabled?: boolean;
    /** 联网搜索开关（受控）。 */
    webSearch?: boolean;
  }>(),
  { disabled: false, webSearch: false },
);

const emit = defineEmits<{
  (e: "send", text: string): void;
  (e: "update:webSearch", v: boolean): void;
}>();

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
      class="composer__web"
      type="button"
      :class="{ 'is-on': webSearch }"
      :title="webSearch ? '联网搜索已开启（消耗更多额度）' : '联网搜索已关闭'"
      :aria-pressed="webSearch"
      @click="emit('update:webSearch', !webSearch)"
    >
      <svg
        class="ico-globe"
        viewBox="0 0 24 24"
        width="15"
        height="15"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <circle
          cx="12"
          cy="12"
          r="9"
        />
        <path d="M3 12h18" />
        <path d="M12 3c2.6 2.7 2.6 15.3 0 18M12 3c-2.6 2.7-2.6 15.3 0 18" />
      </svg>
      <span>联网</span>
    </button>
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
.composer__web {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 10px;
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  background: var(--surface-panel);
  color: var(--ink-soft);
  font: 600 12px var(--font-ui);
  cursor: pointer;
  transition:
    border-color var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}
.composer__web:hover {
  border-color: var(--peach);
  color: var(--peach-deep, #e07a4f);
}
.composer__web.is-on {
  border-color: transparent;
  background: linear-gradient(135deg, var(--peach, #ffc3a0), var(--butter, #ffe3a3));
  color: var(--ink);
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

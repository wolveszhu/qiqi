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
/* 整行做成抬升的输入条：与消息区背景拉开层次（浅底 + 描边 + 阴影） */
.composer {
  display: flex;
  align-items: stretch;
  gap: 8px;
  padding: 8px;
  border: 1px solid var(--line);
  border-radius: var(--r-md);
  background: var(--bg-window);
  box-shadow: var(--shadow-card);
}
.composer__field {
  flex: 1;
  resize: none;
  border: 1px solid transparent;
  border-radius: var(--r-sm);
  background: var(--bg-soft);
  color: var(--ink);
  padding: 9px 11px;
  font: 500 13px/1.4 var(--font-body);
  outline: none;
  transition: border-color var(--dur-fast) var(--ease-out);
}
.composer__field::placeholder {
  color: var(--ink-soft);
}
.composer__field:focus {
  border-color: var(--matcha);
}
.composer__field:disabled {
  opacity: 0.6;
}
/* 联网开关：清晰胶囊；关=描边灰、开=蜜桃渐变实底（最醒目） */
.composer__web {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-radius: var(--r-sm);
  background: var(--bg-soft);
  color: var(--ink-soft);
  font: 600 13px var(--font-body);
  cursor: pointer;
  transition:
    border-color var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}
.composer__web:hover {
  border-color: var(--peach);
  color: var(--peach);
}
.composer__web.is-on {
  border-color: transparent;
  background: linear-gradient(135deg, var(--peach), var(--butter));
  color: var(--ink);
  box-shadow: var(--shadow-card);
}
/* 发送：主色抹茶绿实底 + 阴影，最醒目 */
.composer__send {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 20px;
  border: none;
  border-radius: var(--r-sm);
  background: var(--matcha-deep);
  color: #fff;
  font: 700 14px var(--font-body);
  cursor: pointer;
  box-shadow: var(--shadow-card);
  transition:
    filter var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}
.composer__send:hover:not(:disabled) {
  background: var(--matcha);
}
.composer__send:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  box-shadow: none;
}
.ico-globe {
  flex-shrink: 0;
}
</style>

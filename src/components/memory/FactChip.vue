<script setup lang="ts">
/**
 * 事实标签（设计方案 §5.2 / §4.5）：内容 + conf 置信度微胶囊 + ✎ 编辑 + ✕ 删除（二次确认，hover 变腮红）。
 * 编辑=行内受控 input，回车/失焦保存、Esc 取消（零副作用）；删除=点击转确认态（✕→「确认?」）防误删。
 * 均上抛事件，写操作在父级。
 */
import { ref } from "vue";

import type { FactDto } from "../../api/types";

const props = defineProps<{ fact: FactDto }>();
const emit = defineEmits<{
  (e: "edit", id: number, content: string): void;
  (e: "delete", id: number): void;
}>();

const editing = ref(false);
const draft = ref(props.fact.content);
const confirming = ref(false);

function confLabel(c: number): string {
  const pct = c <= 1 ? Math.round(c * 100) : Math.round(c);
  return `${Math.max(0, Math.min(100, pct))}%`;
}

function startEdit(): void {
  editing.value = true;
  draft.value = props.fact.content;
}
function commitEdit(): void {
  editing.value = false;
  const next = draft.value.trim();
  if (next && next !== props.fact.content) emit("edit", props.fact.id, next);
}
function cancelEdit(): void {
  // Esc 取消：恢复草稿为原值，blur 时 next===content 不落库（零副作用）
  draft.value = props.fact.content;
  editing.value = false;
}
function onDel(): void {
  if (!confirming.value) {
    confirming.value = true;
    return;
  }
  emit("delete", props.fact.id);
}
</script>

<template>
  <div
    class="fact-chip"
    :class="{ 'is-editing': editing }"
  >
    <input
      v-if="editing"
      v-model="draft"
      class="fact-chip__input"
      @keyup.enter="commitEdit"
      @keyup.esc="cancelEdit"
      @blur="commitEdit"
    />
    <template v-else>
      <span class="fact-chip__content">{{ fact.content }}</span>
      <span class="fact-chip__conf">conf {{ confLabel(fact.confidence) }}</span>
      <button
        class="fact-chip__btn"
        type="button"
        aria-label="编辑"
        @click="startEdit"
      >
        ✎
      </button>
      <button
        class="fact-chip__btn fact-chip__btn--del"
        :class="{ 'is-confirm': confirming }"
        type="button"
        aria-label="删除"
        @click="onDel"
      >
        {{ confirming ? "确认?" : "✕" }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.fact-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  background: var(--bg-window);
  border: 1px solid var(--line);
  border-radius: 999px;
  max-width: 100%;
}
.fact-chip__content {
  font-size: 12px;
  color: var(--ink);
}
.fact-chip__conf {
  font-size: 9px;
  font-weight: 900;
  color: var(--matcha-deep);
  background: var(--matcha-soft);
  padding: 1px 6px;
  border-radius: 999px;
  flex: none;
}
.fact-chip__input {
  border: 1px solid var(--matcha);
  border-radius: 8px;
  padding: 2px 8px;
  font-size: 12px;
  color: var(--ink);
  background: var(--bg-window);
  min-width: 140px;
}
.fact-chip__btn {
  width: 19px;
  height: 19px;
  border: none;
  border-radius: 50%;
  background: var(--bg-deep);
  color: var(--ink-soft);
  font-size: 11px;
  cursor: pointer;
  flex: none;
  display: grid;
  place-items: center;
}
.fact-chip__btn:hover {
  color: var(--ink);
}
.fact-chip__btn--del:hover {
  background: #fbe3e7;
  color: var(--peach);
}
.fact-chip__btn--del.is-confirm {
  background: var(--peach);
  color: #fff;
}
</style>

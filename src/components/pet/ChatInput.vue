<script setup lang="ts">
import { nextTick, ref, watch } from "vue";

import { setClickThroughEnabled } from "../../api/window";

const props = defineProps<{ visible: boolean }>();
const emit = defineEmits<{
  (e: "send", text: string): void;
  (e: "close"): void;
}>();

const draft = ref("");
const el = ref<HTMLTextAreaElement | null>(null);

// D1.2 穿透状态机：输入框显示 → 整窗接收（false）；隐藏 → 恢复穿透（true）
watch(
  () => props.visible,
  async (v) => {
    await setClickThroughEnabled(!v);
    if (v) {
      await nextTick();
      el.value?.focus();
    }
  },
);

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    emit("close");
    return;
  }
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    submit();
  }
}

function submit(): void {
  const t = draft.value.trim();
  if (!t) return;
  emit("send", t);
  draft.value = "";
}
</script>

<template>
  <transition name="slide-up">
    <div
      v-if="visible"
      class="chat-input"
    >
      <textarea
        ref="el"
        v-model="draft"
        class="field"
        rows="2"
        placeholder="说点什么…（Enter 发送 / Shift+Enter 换行 / Esc 关闭）"
        @keydown="onKeydown"
      ></textarea>
      <button
        class="send"
        type="button"
        @click="submit"
      >
        发送
      </button>
    </div>
  </transition>
</template>

<style scoped>
.chat-input {
  position: absolute;
  left: 50%;
  bottom: 10%;
  transform: translateX(-50%);
  width: 86%;
  display: flex;
  gap: 8px;
  z-index: 20;
}
.field {
  flex: 1;
  resize: none;
  border-radius: 12px;
  border: 1px solid var(--border-soft);
  background: var(--surface-panel);
  color: var(--text-primary);
  padding: 8px 10px;
  font: 500 13px/1.4 var(--font-ui);
  outline: none;
}
.field:focus {
  border-color: var(--accent-rose);
}
.send {
  align-self: stretch;
  padding: 0 14px;
  border: none;
  border-radius: 12px;
  background: var(--accent-rose);
  color: #fff;
  font: 600 13px var(--font-ui);
  cursor: pointer;
}
.send:hover {
  filter: brightness(1.05);
}
.slide-up-enter-active,
.slide-up-leave-active {
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
}
.slide-up-enter-from,
.slide-up-leave-to {
  opacity: 0;
  transform: translate(-50%, 8px);
}
</style>

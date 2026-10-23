<script setup lang="ts">
import { nextTick, ref, watch } from "vue";

const props = defineProps<{ visible: boolean }>();
const emit = defineEmits<{
  (e: "send", text: string): void;
  (e: "close"): void;
}>();

const draft = ref("");
const el = ref<HTMLTextAreaElement | null>(null);

/**
 * 聚焦输入区（穿透总闸与窗口聚焦由 PetWindow 统一处理，此处只管键盘焦点）。
 * 窗口展开（set_size）会让 webview 失焦，首次 focus 可能落空 → 下一帧重试一次。
 */
function focusField(): void {
  const field = el.value;
  if (!field) return;
  field.focus();
  if (document.activeElement !== field) {
    requestAnimationFrame(() => el.value?.focus());
  }
}

watch(
  () => props.visible,
  async (v) => {
    if (!v) return;
    await nextTick();
    focusField();
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
      <button
        class="close"
        type="button"
        aria-label="关闭输入框"
        @click="emit('close')"
      >
        ✕
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
.close {
  align-self: stretch;
  padding: 0 10px;
  border: none;
  border-radius: 12px;
  background: var(--surface-panel);
  color: var(--ink-soft);
  font: 600 13px var(--font-ui);
  cursor: pointer;
}
.close:hover {
  background: var(--peach-soft);
  color: var(--peach);
}
.slide-up-enter-active,
.slide-up-leave-active {
  transition:
    opacity var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}
.slide-up-enter-from,
.slide-up-leave-to {
  opacity: 0;
  transform: translate(-50%, 8px);
}
</style>

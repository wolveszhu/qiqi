<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import ChatInput from "../../components/pet/ChatInput.vue";
import SpeechBubble from "../../components/pet/SpeechBubble.vue";
import { setClickThroughEnabled } from "../../api/window";
import { resolveDroppedFiles } from "../../api/chat";
import { useChatStream } from "../../composables/useChatStream";
import { setIdleMode, usePetRenderer } from "../../composables/usePetRenderer";
import { proactiveBubble, useProactive } from "../../composables/useProactive";
import { useSystemProbe } from "../../composables/useSystemProbe";
import { chatStore } from "../../state/chatStore";
import type { FilePayload } from "../../api/types";

const { playAction, lookAt } = usePetRenderer();
const { send } = useChatStream();
// 主动互动编排（C6）：自动订阅 + 5min tick；proactiveBubble 供气泡渲染
useProactive();

// C6.5 空闲睡/醒（保留，不破坏主动互动链路）
useSystemProbe({
  onIdleChange: (idle) => {
    if (idle) {
      void playAction("sleep");
      setIdleMode(true);
    } else {
      void playAction("stretch");
      setIdleMode(false);
    }
  },
});

// ---- D1 状态 ----
const inputVisible = ref(false);
const attachments = ref<FilePayload[]>([]);
const dragOver = ref(false);
const notice = ref("");
let noticeTimer: ReturnType<typeof setTimeout> | null = null;

const unlisteners: UnlistenFn[] = [];

/** 气泡优先级：对话流 > 主动话术（D1.4 流式覆盖主动气泡）。 */
const bubbleText = computed(() =>
  chatStore.isStreaming || chatStore.text ? chatStore.text : proactiveBubble.text,
);
const bubbleStreaming = computed(() => chatStore.isStreaming);

function showNotice(msg: string): void {
  notice.value = msg;
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (notice.value = ""), 4000);
}

// ---- D1.2 快捷键显隐输入框（C2 emit window:chat:toggle） ----
// ---- D1.5 拖入多模态（Tauri 内置 tauri://drag-drop 事件） ----
onMounted(async () => {
  unlisteners.push(
    await listen<void>("window:chat:toggle", () => {
      inputVisible.value = !inputVisible.value;
    }),
    await listen<{ paths: string[] }>("tauri://drag-drop", (e) => {
      void onDropFiles(e.payload.paths);
    }),
  );
});

onUnmounted((): void => {
  for (const u of unlisteners.splice(0)) u();
  void setClickThroughEnabled(true); // 卸载复位穿透（防残留整窗接收）
  if (noticeTimer) clearTimeout(noticeTimer);
});

async function onDropFiles(paths: string[]): Promise<void> {
  try {
    const files = await resolveDroppedFiles(paths);
    attachments.value.push(...files);
  } catch (e) {
    showNotice(e instanceof Error ? e.message : "文件读取失败");
  }
}

function onSend(text: string): void {
  send(text, attachments.value);
  attachments.value = [];
}

function removeAttachment(idx: number): void {
  attachments.value.splice(idx, 1);
}

function onCloseInput(): void {
  inputVisible.value = false;
}
</script>

<template>
  <div
    class="pet-root"
    @pointermove="(e) => lookAt(e.clientX, e.clientY)"
    @pointerdown="() => playAction('click')"
    @dragover.prevent="dragOver = true"
    @dragleave="dragOver = false"
  >
    <canvas id="pet-canvas"></canvas>

    <!-- D1.1 气泡（打字机/防截断/靠右缘左翻） -->
    <SpeechBubble
      :text="bubbleText"
      :streaming="bubbleStreaming"
      :visible="true"
    />

    <!-- D1.5 拖入附件提示条 -->
    <div
      v-if="attachments.length"
      class="attach-bar"
    >
      <span
        v-for="(a, i) in attachments"
        :key="a.name"
        class="chip"
        :class="a.kind"
        @click="removeAttachment(i)"
      >
        {{ a.kind === "image" ? "🖼" : "📄" }} {{ a.name }}
        <span class="x">×</span>
      </span>
    </div>

    <!-- D1.5 拒绝提示 -->
    <transition name="fade">
      <div
        v-if="notice"
        class="notice"
      >
        {{ notice }}
      </div>
    </transition>

    <!-- D1.2 输入框（Ctrl+Shift+P 唤出；穿透状态机在组件内） -->
    <ChatInput
      :visible="inputVisible"
      @send="onSend"
      @close="onCloseInput"
    />

    <!-- 拖拽高亮（D1.5 视觉反馈） -->
    <div
      v-if="dragOver"
      class="drop-mask"
    >
      松开以发送图片或文本
    </div>
  </div>
</template>

<style scoped>
.pet-root {
  position: relative;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: transparent;
}
#pet-canvas {
  width: 100%;
  height: 100%;
  display: block;
}
.attach-bar {
  position: absolute;
  left: 50%;
  bottom: 26%;
  transform: translateX(-50%);
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  justify-content: center;
  max-width: 90%;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 999px;
  background: var(--surface-panel);
  color: var(--text-primary);
  font: 500 12px var(--font-ui);
  cursor: pointer;
  box-shadow: 0 2px 6px var(--shadow-soft);
}
.chip.image {
  border: 1px solid var(--accent-rose);
}
.chip .x {
  opacity: 0.6;
}
.notice {
  position: absolute;
  left: 50%;
  top: 8%;
  transform: translateX(-50%);
  padding: 6px 12px;
  border-radius: 10px;
  background: #ffe3e3;
  color: #b42318;
  font: 600 12px var(--font-ui);
  z-index: 30;
}
.drop-mask {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(255, 209, 217, 0.18);
  border: 2px dashed var(--accent-rose);
  color: var(--text-primary);
  font: 600 14px var(--font-ui);
  z-index: 5;
  pointer-events: none;
}
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>

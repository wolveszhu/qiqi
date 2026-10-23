<script setup lang="ts">
/**
 * 窗口 B（设置层，设计方案 §3）：自绘 TitleBar + SideNav + 五面板切换。
 * TODO(B1): 关闭=hide（CloseRequested 拦截在 Rust）；TODO(D2): 面板实装。
 */

import { onMounted, onUnmounted, ref } from "vue";

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { focusMainWindow, hide } from "../../api/window";
import SideNav from "../../components/window/SideNav.vue";
import TitleBar from "../../components/window/TitleBar.vue";
import AffectionPanel from "./panels/AffectionPanel.vue";
import AvatarPanel from "./panels/AvatarPanel.vue";
import ChatPanel from "./panels/ChatPanel.vue";
import DndPanel from "./panels/DndPanel.vue";
import LlmPanel from "./panels/LlmPanel.vue";
import MemoryPanel from "./panels/MemoryPanel.vue";
import AppToast from "../../components/ui/AppToast.vue";

type PanelKey = "chat" | "avatar" | "llm" | "dnd" | "affection" | "memory";

const active = ref<PanelKey>("avatar");
const unlisteners: UnlistenFn[] = [];

/**
 * ✕ → `window_hide("settings")`（关闭=hide，进程存活；B1.4 临时通道已由
 * C2 的 `api/window.ts` 正式命令组转正）。
 */
async function closeWindow(): Promise<void> {
  try {
    await hide("settings");
  } catch (e) {
    console.error("[settings] 隐藏窗口失败", e);
  }
}

onMounted(async (): Promise<void> => {
  // 窗口 A 双击形象 / Ctrl+Shift+P → 切到对话 Tab 并聚焦（A 只负责 show，其余在此完成）
  unlisteners.push(
    await listen("window:chat:open", () => {
      active.value = "chat";
      void focusMainWindow().catch(() => {});
    }),
  );
});

onUnmounted((): void => {
  unlisteners.forEach((fn) => fn());
});
</script>

<template>
  <div class="settings-window">
    <TitleBar @close="closeWindow" />
    <div class="settings-window__body">
      <SideNav
        :active="active"
        @change="(k: PanelKey) => (active = k)"
      />
      <main class="settings-window__content">
        <ChatPanel v-if="active === 'chat'" />
        <AvatarPanel v-else-if="active === 'avatar'" />
        <LlmPanel v-else-if="active === 'llm'" />
        <DndPanel v-else-if="active === 'dnd'" />
        <AffectionPanel v-else-if="active === 'affection'" />
        <MemoryPanel v-else />
      </main>
    </div>
    <AppToast />
  </div>
</template>

<style scoped>
.settings-window {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background: var(--bg-page);
}

.settings-window__body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.settings-window__content {
  flex: 1;
  padding: 24px 28px;
  overflow: hidden;
}
</style>

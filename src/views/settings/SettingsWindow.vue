<script setup lang="ts">
/**
 * 窗口 B（设置层，设计方案 §3）：自绘 TitleBar + SideNav + 五面板切换。
 * TODO(B1): 关闭=hide（CloseRequested 拦截在 Rust）；TODO(D2): 面板实装。
 */

import { ref } from "vue";

import { hide } from "../../api/window";
import SideNav from "../../components/window/SideNav.vue";
import TitleBar from "../../components/window/TitleBar.vue";
import AffectionPanel from "./panels/AffectionPanel.vue";
import AvatarPanel from "./panels/AvatarPanel.vue";
import DndPanel from "./panels/DndPanel.vue";
import LlmPanel from "./panels/LlmPanel.vue";
import MemoryPanel from "./panels/MemoryPanel.vue";

type PanelKey = "avatar" | "llm" | "dnd" | "affection" | "memory";

const active = ref<PanelKey>("avatar");

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
        <AvatarPanel v-if="active === 'avatar'" />
        <LlmPanel v-else-if="active === 'llm'" />
        <DndPanel v-else-if="active === 'dnd'" />
        <AffectionPanel v-else-if="active === 'affection'" />
        <MemoryPanel v-else />
      </main>
    </div>
  </div>
</template>

<style scoped>
.settings-window {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
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
  overflow-y: auto;
}
</style>

<script setup lang="ts">
/**
 * 窗口 B（设置层，设计方案 §3）：自绘 TitleBar + SideNav + 五面板切换。
 * TODO(B1): 关闭=hide（CloseRequested 拦截在 Rust）；TODO(D2): 面板实装。
 */

import { onMounted, onUnmounted, ref } from "vue";

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

import { normalizeError } from "../../api/client";
import { focusMainWindow, hide } from "../../api/window";
import SideNav from "../../components/window/SideNav.vue";
import TitleBar from "../../components/window/TitleBar.vue";
import AdBanner from "../../components/settings/AdBanner.vue";
import AdCard from "../../components/settings/AdCard.vue";
import UpdateNotice from "../../components/settings/UpdateNotice.vue";
import AffectionPanel from "./panels/AffectionPanel.vue";
import AvatarPanel from "./panels/AvatarPanel.vue";
import ChatPanel from "./panels/ChatPanel.vue";
import DndPanel from "./panels/DndPanel.vue";
import LlmPanel from "./panels/LlmPanel.vue";
import MemoryPanel from "./panels/MemoryPanel.vue";
import AboutPanel from "./panels/AboutPanel.vue";
import AppToast from "../../components/ui/AppToast.vue";
import { initAds, useAds } from "../../composables/useAds";
import { initUpdateWatcher, useUpdate } from "../../composables/useUpdate";
import { pushToast } from "../../composables/useToast";
import { SIDEBAR_CARD_AD, TOP_BANNER_AD } from "../../config/ads";

type PanelKey = "chat" | "avatar" | "llm" | "dnd" | "affection" | "memory" | "about";

const active = ref<PanelKey>("avatar");
const unlisteners: UnlistenFn[] = [];
const { hasUpdate } = useUpdate();
const { isVisible, dismissAd } = useAds();

/** 打开广告跳转链接（系统浏览器；失败 toast 不崩溃）。 */
async function openAd(link: string): Promise<void> {
  try {
    await openUrl(link);
  } catch (e: unknown) {
    pushToast(normalizeError(e).message, "error");
  }
}

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
  // 启动更新监听（后台周期检查发现新版本 → 横幅/Tab 标记）
  initUpdateWatcher();
  // 加载广告关闭状态（今日不再显示；失败回落全部可见）
  void initAds();
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
    <UpdateNotice />
    <AdBanner
      v-if="isVisible(TOP_BANNER_AD.id)"
      :ad="TOP_BANNER_AD"
      @open="openAd(TOP_BANNER_AD.link)"
      @dismiss="dismissAd(TOP_BANNER_AD.id)"
    />
    <div class="settings-window__body">
      <SideNav
        :active="active"
        :has-update="hasUpdate"
        @change="(k: PanelKey) => (active = k)"
      >
        <template #footer>
          <AdCard
            v-if="isVisible(SIDEBAR_CARD_AD.id)"
            :ad="SIDEBAR_CARD_AD"
            @open="openAd(SIDEBAR_CARD_AD.link)"
            @dismiss="dismissAd(SIDEBAR_CARD_AD.id)"
          />
        </template>
      </SideNav>
      <main class="settings-window__content scroll-area">
        <!-- KeepAlive：切 Tab 不卸载面板——对话流式期间离开对话 Tab 也能继续收流 -->
        <KeepAlive>
          <ChatPanel v-if="active === 'chat'" />
          <AvatarPanel v-else-if="active === 'avatar'" />
          <LlmPanel v-else-if="active === 'llm'" />
          <DndPanel v-else-if="active === 'dnd'" />
          <AffectionPanel v-else-if="active === 'affection'" />
          <MemoryPanel v-else-if="active === 'memory'" />
          <AboutPanel v-else />
        </KeepAlive>
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
  min-height: 0; /* flex 子项允许收缩，溢出才交给 .scroll-area 滚动而非撑高父级 */
  padding: 24px 28px;
}
</style>

<script setup lang="ts">
/**
 * 全局更新提醒横幅（非阻塞、可关闭）。
 *
 * 后台周期检查发现新版本时（`app:update-available`）由 `useUpdate` 单例置位 `hasUpdate`，
 * 此处展示在设置窗口顶部；关闭仅隐藏横幅与 Tab 标记，「关于」面板内的信息仍在。
 */

import { useI18n } from "vue-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";

import { useUpdate } from "../../composables/useUpdate";

const { t } = useI18n();
const { updateInfo, hasUpdate, dismiss } = useUpdate();

async function download(): Promise<void> {
  const url = updateInfo.value?.downloadUrl;
  if (url) await openUrl(url);
}
</script>

<template>
  <div
    v-if="hasUpdate"
    class="update-notice"
  >
    <span class="update-notice__dot"></span>
    <span class="update-notice__text">
      {{ t("updateNotice.found", { v: updateInfo?.latest ?? "" }) }}
    </span>
    <button
      class="update-notice__btn"
      type="button"
      @click="download"
    >
      {{ t("updateNotice.download") }}
    </button>
    <button
      class="update-notice__close"
      type="button"
      :aria-label="t('updateNotice.later')"
      @click="dismiss"
    >
      ✕
    </button>
  </div>
</template>

<style scoped>
.update-notice {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 12px 16px 0;
  padding: 10px 14px;
  background: var(--peach-soft);
  border: 1px solid var(--peach);
  border-radius: var(--r-md);
  box-shadow: var(--shadow-card);
  animation: notice-in var(--dur-fast) var(--ease-out);
}
.update-notice__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--peach);
  flex-shrink: 0;
}
.update-notice__text {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: var(--ink);
}
.update-notice__btn {
  border: none;
  border-radius: var(--r-sm);
  padding: 6px 14px;
  background: var(--peach);
  color: var(--bg-window);
  font: 600 13px var(--font-body);
  cursor: pointer;
  transition: filter var(--dur-instant) var(--ease-out);
}
.update-notice__btn:hover {
  filter: brightness(1.06);
}
.update-notice__close {
  border: none;
  background: none;
  color: var(--ink-soft);
  font-size: 13px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 6px;
}
.update-notice__close:hover {
  background: var(--bg-window);
  color: var(--ink);
}
@keyframes notice-in {
  from {
    opacity: 0;
    transform: translateY(-6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>

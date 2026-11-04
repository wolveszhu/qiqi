<script setup lang="ts">
/**
 * 顶部文字广告横幅（纯 UI：文字 + 背景色 + 跳转链接，无图片）。
 *
 * 业务动作由父级处理：`open`（跳转系统浏览器）/ `dismiss`（今日不再显示），
 * 组件自身零业务依赖、不发 IPC，便于隔离测试。
 */

import type { AdItem } from "../../config/ads";

defineProps<{ ad: AdItem }>();

const emit = defineEmits<{
  (e: "open"): void;
  (e: "dismiss"): void;
}>();
</script>

<template>
  <div
    class="ad-banner"
    :class="`ad-banner--${ad.tone ?? 'matcha'}`"
  >
    <span class="ad-banner__tag">广告</span>
    <span class="ad-banner__text">{{ ad.text }}</span>
    <button
      class="ad-banner__link"
      type="button"
      @click="emit('open')"
    >
      了解详情 →
    </button>
    <button
      class="ad-banner__close"
      type="button"
      aria-label="关闭广告"
      @click="emit('dismiss')"
    >
      ✕
    </button>
  </div>
</template>

<style scoped>
.ad-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 36px;
  margin: 12px 16px 0;
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: var(--r-sm);
  animation: banner-in var(--dur-fast) var(--ease-out);
}
.ad-banner--matcha {
  background: var(--matcha-soft);
}
.ad-banner--peach {
  background: var(--peach-soft);
}
.ad-banner__tag {
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 700;
  color: var(--ink);
  background: var(--butter);
  padding: 2px 8px;
  border-radius: 999px;
}
.ad-banner__text {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  color: var(--ink);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ad-banner__link {
  flex-shrink: 0;
  border: none;
  background: none;
  padding: 4px 2px;
  font: 600 12px var(--font-body);
  color: var(--matcha-deep);
  cursor: pointer;
}
.ad-banner__link:hover {
  text-decoration: underline;
}
.ad-banner__close {
  flex-shrink: 0;
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 6px;
  background: none;
  color: var(--ink-soft);
  font-size: 12px;
  cursor: pointer;
}
.ad-banner__close:hover {
  background: var(--bg-window);
  color: var(--ink);
}
@keyframes banner-in {
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

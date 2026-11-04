<script setup lang="ts">
/**
 * 侧边栏图片广告卡（纯 UI）。
 *
 * 素材规格 300×250（IAB medium rectangle），显示宽 144px 等比缩放为 144×120。
 * 业务动作由父级处理：`open`（跳转系统浏览器）/ `dismiss`（今日不再显示）。
 */

import type { AdItem } from "../../config/ads";

defineProps<{ ad: AdItem }>();

const emit = defineEmits<{
  (e: "open"): void;
  (e: "dismiss"): void;
}>();
</script>

<template>
  <div class="ad-card">
    <button
      class="ad-card__hit"
      type="button"
      aria-label="查看广告详情"
      @click="emit('open')"
    >
      <img
        class="ad-card__img"
        :src="ad.image"
        width="144"
        height="120"
        alt="广告"
      />
      <span class="ad-card__tag">广告</span>
    </button>
    <button
      class="ad-card__close"
      type="button"
      aria-label="关闭广告"
      @click="emit('dismiss')"
    >
      ✕
    </button>
  </div>
</template>

<style scoped>
.ad-card {
  position: relative;
  width: 144px;
  margin-top: auto;
  border-radius: var(--r-sm);
  animation: card-in var(--dur-fast) var(--ease-out);
}
.ad-card__hit {
  display: block;
  width: 100%;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: var(--r-sm);
  background: var(--bg-window);
  cursor: pointer;
  overflow: hidden;
  transition:
    transform var(--dur-fast) var(--ease-out),
    box-shadow var(--dur-fast) var(--ease-out);
}
.ad-card__hit:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-card);
}
.ad-card__img {
  display: block;
  width: 144px;
  height: 120px;
  object-fit: cover;
}
.ad-card__tag {
  position: absolute;
  top: 6px;
  left: 6px;
  font-size: 10px;
  color: #fff;
  background: rgb(64 58 46 / 65%);
  padding: 1px 6px;
  border-radius: 999px;
}
.ad-card__close {
  position: absolute;
  top: 5px;
  right: 5px;
  display: grid;
  place-items: center;
  width: 18px;
  height: 18px;
  border: none;
  border-radius: 6px;
  color: var(--ink);
  background: rgb(255 254 249 / 85%);
  font-size: 11px;
  cursor: pointer;
  opacity: 0.55;
  transition: opacity var(--dur-instant) var(--ease-out);
}
.ad-card__close:hover {
  opacity: 1;
}
@keyframes card-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>

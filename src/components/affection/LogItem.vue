<script setup lang="ts">
/**
 * 好感度流水条目（设计方案 §5.2 / §4.4）：左侧色位 + emoji，中部原因/时间，右侧 mono delta。
 * 纯展示；衰减/禁忌条目文案情绪化由后端 reason 字段保证（前端如实呈现）。
 */
import { computed } from "vue";

import type { LogItem } from "../../api/types";

const props = defineProps<{ log: LogItem }>();

const isPlus = computed(() => props.log.changeValue >= 0);

const emoji = computed((): string => {
  const r = props.log.reason;
  if (r.includes("禁忌")) return "😾";
  if (r.includes("衰减")) return "💤";
  if (r.includes("打卡")) return "🌞";
  if (r.includes("抚摸")) return "🐾";
  if (r.includes("共鸣") || r.includes("深度")) return "💬";
  return "✨";
});

function fmtTime(s: string): string {
  // 兼容 ISO（T 分隔）与 "YYYY-MM-DD HH:MM:SS"
  return s.replace("T", " ").slice(0, 16);
}
</script>

<template>
  <div class="log-item">
    <span
      class="log-item__icon"
      :class="isPlus ? 'is-plus' : 'is-minus'"
      >{{ emoji }}</span
    >
    <div class="log-item__body">
      <div class="log-item__reason">{{ log.reason }}</div>
      <div class="log-item__time">{{ fmtTime(log.createdAt) }}</div>
    </div>
    <span
      class="log-item__delta"
      :class="isPlus ? 'plus' : 'minus'"
      >{{ isPlus ? "+" : "" }}{{ log.changeValue }}</span
    >
  </div>
</template>

<style scoped>
.log-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 12px;
}
.log-item__icon {
  width: 32px;
  height: 32px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 16px;
  flex: none;
}
.log-item__icon.is-plus {
  background: var(--matcha-soft);
}
.log-item__icon.is-minus {
  background: #fbe3e7;
}
.log-item__body {
  flex: 1;
  min-width: 0;
}
.log-item__reason {
  font-size: 13px;
  color: var(--ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.log-item__time {
  font-size: 11px;
  color: var(--ink-soft);
  font-family: Consolas, monospace;
}
.log-item__delta {
  font-family: Consolas, monospace;
  font-size: 13px;
  font-weight: 700;
  flex: none;
}
.log-item__delta.plus {
  color: var(--matcha-deep);
}
.log-item__delta.minus {
  color: var(--peach);
}
</style>

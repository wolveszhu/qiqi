<script setup lang="ts">
/**
 * 好感度流水条目（设计方案 §5.2 / §4.4）：左侧色位 + emoji，中部原因/时间，右侧 mono delta。
 * 纯展示；reason 为后端英文 key，经 `utils/affection` 的 `reasonMeta` 译为中文。
 */
import { computed } from "vue";

import { reasonMeta } from "../../utils/affection";
import { useAffectionI18n } from "../../composables/useAffectionI18n";
import type { LogItem } from "../../api/types";

const props = defineProps<{ log: LogItem }>();

const { reasonLabel } = useAffectionI18n();

const isPlus = computed(() => props.log.changeValue >= 0);

/** reason（后端英文 key）→ 本地化文案 + emoji（emoji 仍来自 utils 单一真源）。 */
const meta = computed(() => ({
  label: reasonLabel(props.log.reason),
  emoji: reasonMeta(props.log.reason).emoji,
}));

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
      >{{ meta.emoji }}</span
    >
    <div class="log-item__body">
      <div class="log-item__reason">{{ meta.label }}</div>
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

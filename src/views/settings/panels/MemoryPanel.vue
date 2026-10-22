<script setup lang="ts">
/**
 * 记忆面板（设计方案 §4.5，D3 范围）：D2 仅接 `memory_list_diaries` 空态，避免走查空白报错；
 * 日记卡片 / 事实标签编辑的完整交互留待 D3。有日记时给轻量预览（日期 + 首行）。
 */
import { onMounted, ref } from "vue";

import AppCard from "../../../components/ui/AppCard.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import { listDiaries } from "../../../api/memory";
import type { DiaryDto } from "../../../api/types";

const diaries = ref<DiaryDto[]>([]);
const loading = ref(false);

function preview(text: string): string {
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length > 60 ? `${flat.slice(0, 60)}…` : flat;
}

onMounted(async (): Promise<void> => {
  loading.value = true;
  try {
    diaries.value = await listDiaries();
  } catch (e) {
    console.warn("[memory] 日记拉取失败", e);
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <AppCard title="记忆">
    <HintBox> 记忆阅读区将在后续版本开放完整编辑；当前先预览七七写过的日记。 </HintBox>

    <div
      v-if="loading"
      class="mem-loading"
    >
      正在读取日记…
    </div>

    <ul
      v-else-if="diaries.length"
      class="mem-list"
    >
      <li
        v-for="d in diaries"
        :key="d.id"
        class="mem-item"
      >
        <span class="mem-item__date">{{ d.diaryDate }}</span>
        <span class="mem-item__text">{{ preview(d.content) }}</span>
      </li>
    </ul>

    <HintBox
      v-else
      tone="info"
    >
      她还没写过日记——今晚见。
    </HintBox>
  </AppCard>
</template>

<style scoped>
.mem-loading {
  font-size: 13px;
  color: var(--ink-soft);
}
.mem-list {
  list-style: none;
  margin: 12px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.mem-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px;
  background: var(--bg-soft);
  border-radius: 12px;
}
.mem-item__date {
  font-size: 11px;
  font-family: Consolas, monospace;
  color: var(--ink-soft);
}
.mem-item__text {
  font-size: 13px;
  color: var(--ink);
}
</style>

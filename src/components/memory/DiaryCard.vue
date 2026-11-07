<script setup lang="ts">
/**
 * 日记卡片（设计方案 §5.2 / §4.5）：📖 头 + 日期 + 超大引号水印；第一人称正文；
 * foot 心情 badge（green）+ 重新生成（ghost 小按钮，二次确认）；生成中 loading 防重复点击。
 * mood 映射走 `constants/mood`（后端 0=开心/1=平静/2=低落，null=未记录）。
 */
import { ref } from "vue";
import { useI18n } from "vue-i18n";

import AppButton from "../../components/ui/AppButton.vue";
import StatusBadge from "../../components/ui/StatusBadge.vue";
import type { DiaryDto, DiaryMood } from "../../api/types";
import { moodView } from "../../constants/mood";

const { t } = useI18n();

const props = withDefaults(defineProps<{ diary: DiaryDto; loading?: boolean }>(), {
  loading: false,
});
const emit = defineEmits<{ (e: "regenerate", date: string): void }>();

const confirming = ref(false);

/** 情绪文案键（数值来自后端契约 0/1/2，null=平静兜底）。 */
const MOOD_KEYS: Record<DiaryMood, string> = {
  0: "diary.moodHappy",
  1: "diary.moodCalm",
  2: "diary.moodLow",
};

function moodLabel(m: DiaryMood | null): string {
  const v = moodView(m);
  const key = m == null ? "diary.moodCalm" : (MOOD_KEYS[m] ?? "diary.moodCalm");
  return `${v.emoji} ${t(key)}`;
}

function onRegen(): void {
  if (props.loading) return;
  if (!confirming.value) {
    confirming.value = true;
    return;
  }
  emit("regenerate", props.diary.diaryDate);
}
</script>

<template>
  <article class="diary-card">
    <span
      class="diary-card__quote"
      aria-hidden="true"
      >”</span
    >
    <header class="diary-card__head">
      <span class="diary-card__icon">📖</span>
      <span class="diary-card__title">{{ t("memory.diary") }}</span>
      <span class="diary-card__date">{{ diary.diaryDate }}</span>
    </header>
    <p class="diary-card__body">{{ diary.content }}</p>
    <footer class="diary-card__foot">
      <StatusBadge
        tone="ok"
        :text="moodLabel(diary.mood)"
      />
      <AppButton
        variant="ghost"
        :loading="loading"
        :class="{ 'is-confirm': confirming }"
        @click="onRegen"
      >
        {{ confirming ? t("diary.confirmRegen") : t("diary.regen") }}
      </AppButton>
    </footer>
  </article>
</template>

<style scoped>
.diary-card {
  position: relative;
  padding: 14px 16px;
  background: var(--bg-soft);
  border-radius: 16px;
  overflow: hidden;
}
.diary-card__quote {
  position: absolute;
  top: -18px;
  right: 10px;
  font-family: Georgia, serif;
  font-size: 110px;
  line-height: 1;
  color: rgba(255, 158, 94, 0.16);
  pointer-events: none;
}
.diary-card__head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.diary-card__icon {
  font-size: 15px;
}
.diary-card__title {
  font-size: 14px;
  font-weight: 700;
  color: var(--ink);
}
.diary-card__date {
  margin-left: auto;
  font-size: 11px;
  font-family: Consolas, monospace;
  color: var(--ink-soft);
}
.diary-card__body {
  margin: 8px 0 12px;
  font-size: 12.5px;
  line-height: 2;
  color: var(--ink);
  white-space: pre-wrap;
  max-height: 220px;
  overflow-y: auto;
}
.diary-card__foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.diary-card :deep(.app-btn) {
  padding: 4px 12px;
  font-size: 12px;
}
.diary-card :deep(.app-btn.is-confirm) {
  color: var(--peach);
  border-color: var(--peach);
}
</style>

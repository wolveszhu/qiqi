<script setup lang="ts">
/**
 * 通用设置面板（i18n 新增）：语言切换。
 *
 * 语言名按惯例以母语展示（中文/English），不随当前语言翻译；
 * 其余文案走 vue-i18n。切换即落库（Rust 重建托盘）并即时刷新本窗口 UI。
 */

import { ref } from "vue";
import { useI18n } from "vue-i18n";

import SegmentedControl from "../../../components/ui/SegmentedControl.vue";
import { useLocale } from "../../../composables/useLocale";
import { pushToast } from "../../../composables/useToast";
import { normalizeError } from "../../../api/client";
import type { Locale } from "../../../i18n";

const { t } = useI18n();
const { currentLocale, switchLocale } = useLocale();

const saving = ref(false);

// 语言名以母语呈现（不随界面语言变化）
const options: { value: Locale; label: string }[] = [
  { value: "zh-CN", label: "中文" },
  { value: "en-US", label: "English" },
];

/** 选择语言：切换落库，失败 toast 不崩溃。 */
async function onSelect(value: Locale): Promise<void> {
  if (value === currentLocale.value) return;
  saving.value = true;
  try {
    await switchLocale(value);
  } catch (e: unknown) {
    pushToast(normalizeError(e).message, "error");
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <section class="general-panel">
    <h2 class="general-panel__title">{{ t("general.title") }}</h2>
    <div class="general-panel__row">
      <div class="general-panel__label">
        <div class="general-panel__name">{{ t("general.language") }}</div>
        <div class="general-panel__desc">{{ t("general.languageDesc") }}</div>
      </div>
      <SegmentedControl
        :model-value="currentLocale"
        :options="options"
        @update:model-value="onSelect"
      />
    </div>
  </section>
</template>

<style scoped>
.general-panel {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.general-panel__title {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  color: var(--ink);
}

.general-panel__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 16px 18px;
  background: var(--bg-soft);
  border: 1px solid var(--line);
  border-radius: var(--r-md);
}

.general-panel__name {
  font-size: 14px;
  font-weight: 600;
  color: var(--ink);
}

.general-panel__desc {
  margin-top: 4px;
  font-size: 12px;
  color: var(--ink-soft);
  max-width: 420px;
  line-height: 1.5;
}
</style>

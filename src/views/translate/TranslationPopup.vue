<script setup lang="ts">
/**
 * 译文浮窗：原文/译文分层展示，提供一键复制与重新翻译。
 */

import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type { TranslateResult } from "../../api/types";

const props = withDefaults(
  defineProps<{ result: TranslateResult | null; busy?: boolean }>(),
  { busy: false },
);

const emit = defineEmits<{
  (e: "copy"): void;
  (e: "retry"): void;
  (e: "close"): void;
}>();

const { t } = useI18n();
const providerLabel = computed((): string =>
  props.result?.isBuiltin ? t("translate.popup.builtin") : t("translate.popup.custom"),
);
</script>

<template>
  <section class="popup">
    <header class="popup__head">
      <span class="popup__tag">{{ providerLabel }}</span>
      <button
        class="popup__icon"
        type="button"
        :aria-label="t('translate.popup.close')"
        :title="t('translate.popup.close')"
        @click="emit('close')"
      >
        ×
      </button>
    </header>

    <div class="popup__block">
      <div class="popup__label">{{ t("translate.popup.source") }}</div>
      <p class="popup__source">{{ result?.sourceText }}</p>
    </div>
    <div class="popup__block">
      <div class="popup__label">{{ t("translate.popup.translation") }}</div>
      <p class="popup__text">{{ result?.translatedText }}</p>
    </div>

    <p
      v-if="result?.quotaExhausted"
      class="popup__hint"
    >
      {{ t("translate.popup.quotaExhausted") }}
    </p>

    <footer class="popup__actions">
      <button
        class="popup__icon"
        type="button"
        :aria-label="t('translate.popup.copy')"
        :title="t('translate.popup.copy')"
        @click="emit('copy')"
      >
        ⧉
      </button>
      <button
        class="popup__icon"
        type="button"
        :aria-label="t('translate.popup.retry')"
        :title="t('translate.popup.retry')"
        :disabled="busy"
        @click="emit('retry')"
      >
        ↻
      </button>
    </footer>
  </section>
</template>

<style scoped>
.popup {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: calc(100vw - 16px);
  max-height: calc(100vh - 16px);
  margin: 8px;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: var(--r-lg);
  background: var(--bg-window);
  box-shadow: var(--shadow-card);
  overflow: auto;
}

.popup__head,
.popup__actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.popup__tag {
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--bg-soft);
  color: var(--ink-soft);
  font-size: 11px;
}

.popup__block {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.popup__label {
  color: var(--ink-soft);
  font-size: 11px;
}

.popup__source,
.popup__text {
  margin: 0;
  word-break: break-word;
  white-space: pre-wrap;
}

.popup__source {
  color: var(--ink-soft);
  font-size: 12px;
}

.popup__text {
  color: var(--ink);
  font-family: var(--font-disp);
  font-size: 15px;
  line-height: 1.55;
}

.popup__hint {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
}

.popup__icon {
  width: 28px;
  height: 28px;
  border: 1px solid var(--line);
  border-radius: 50%;
  background: transparent;
  color: var(--ink);
  cursor: pointer;
  transition: border-color var(--dur-fast) var(--ease-jelly);
}

.popup__icon:hover:not(:disabled) {
  border-color: var(--peach);
}

.popup__icon:disabled {
  opacity: 0.5;
  cursor: wait;
}
</style>

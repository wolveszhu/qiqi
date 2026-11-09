<script setup lang="ts">
/**
 * 选区旁的横向胶囊工具条（参考元宝划词工具栏）：仅展示，点击动作由父级取词。
 *
 * 工具条出现阶段不取词、不注入按键；「翻译」「复制」都在用户主动点击后才发生。
 */

import { useI18n } from "vue-i18n";

withDefaults(defineProps<{ loading?: boolean }>(), { loading: false });

const emit = defineEmits<{
  (e: "translate"): void;
  (e: "copy"): void;
  (e: "close"): void;
}>();

const { t } = useI18n();
</script>

<template>
  <div
    class="toolbar"
    role="toolbar"
    :aria-label="t('translate.toolbar.label')"
  >
    <span
      class="toolbar__logo"
      aria-hidden="true"
    >
      <span class="toolbar__eye"></span>
      <span class="toolbar__eye"></span>
    </span>
    <span
      class="toolbar__divider"
      aria-hidden="true"
    ></span>

    <button
      class="toolbar__action"
      type="button"
      :disabled="loading"
      @click="emit('translate')"
    >
      <span
        class="toolbar__glyph"
        aria-hidden="true"
      >
        文
      </span>
      <span>{{ t("translate.toolbar.translate") }}</span>
    </button>
    <button
      class="toolbar__action"
      type="button"
      :disabled="loading"
      @click="emit('copy')"
    >
      <span
        class="toolbar__glyph"
        aria-hidden="true"
      >
        ⧉
      </span>
      <span>{{ t("translate.toolbar.copy") }}</span>
    </button>

    <span
      v-if="loading"
      class="toolbar__spinner"
      aria-hidden="true"
    ></span>
    <button
      class="toolbar__close"
      type="button"
      :aria-label="t('translate.toolbar.close')"
      :title="t('translate.toolbar.close')"
      @click="emit('close')"
    >
      ×
    </button>
  </div>
</template>

<style scoped>
.toolbar {
  position: absolute;
  top: 8px;
  left: 8px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px 6px 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--bg-window);
  box-shadow: var(--shadow-card);
  color: var(--ink);
  font-family: var(--font-disp);
  white-space: nowrap;
  animation: toolbar-in var(--dur-fast) var(--ease-jelly);
}

@keyframes toolbar-in {
  from {
    opacity: 0;
    transform: translateY(4px);
  }

  to {
    opacity: 1;
    transform: none;
  }
}

.toolbar__logo {
  position: relative;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: var(--matcha-deep);
  flex-shrink: 0;
}

.toolbar__eye {
  position: absolute;
  top: 9px;
  width: 3px;
  height: 4px;
  border-radius: 50%;
  background: #fff;
}

.toolbar__eye:first-child {
  left: 8px;
}

.toolbar__eye:last-child {
  right: 8px;
}

.toolbar__logo::after {
  content: "";
  position: absolute;
  left: 9px;
  bottom: 6px;
  width: 8px;
  height: 4px;
  border-bottom: 2px solid #fff;
  border-radius: 0 0 8px 8px;
}

.toolbar__divider {
  width: 1px;
  height: 18px;
  margin: 0 4px;
  background: var(--line);
  flex-shrink: 0;
}

.toolbar__action {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--ink);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-jelly);
}

.toolbar__action:hover:not(:disabled) {
  background: var(--bg-soft);
}

.toolbar__action:disabled {
  opacity: 0.5;
  cursor: wait;
}

.toolbar__glyph {
  color: var(--matcha-deep);
  font-size: 14px;
  line-height: 1;
}

.toolbar__spinner {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--peach);
  flex-shrink: 0;
}

.toolbar__close {
  width: 24px;
  height: 24px;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--ink-soft);
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
}

.toolbar__close:hover {
  background: var(--bg-soft);
  color: var(--ink);
}
</style>

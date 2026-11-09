<script setup lang="ts">
/**
 * 划词翻译设置面板：总开关、译入语言、服务商与自定义凭据。
 *
 * 红线：明文 Key 只经 IPC 单向流向 Rust；读取只回脱敏指纹；
 * 所有 await IPC 都 try/catch 并按 kind 给本地化提示。
 */

import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import { normalizeError } from "../../../api/client";
import { getTranslateSettings, saveTranslateSettings } from "../../../api/translate";
import type { TranslateProviderId, TranslateTargetLang } from "../../../api/types";
import { pushToast } from "../../../composables/useToast";
import AppToggle from "../../../components/ui/AppToggle.vue";

const { t, te } = useI18n();

const enabled = ref(true);
const targetLang = ref<TranslateTargetLang>("auto");
const provider = ref<TranslateProviderId>("baidu");
const customApiKey = ref("");
const customAppId = ref("");
const hasBuiltin = ref(false);
const fingerprint = ref<string | null>(null);
const saving = ref(false);

const langOptions = computed(() => [
  { value: "auto", label: t("translate.panel.auto") },
  { value: "zh", label: t("translate.panel.zh") },
  { value: "en", label: t("translate.panel.en") },
]);

const providerOptions = computed<Array<{ value: TranslateProviderId; label: string }>>(
  () => [
    { value: "baidu", label: t("translate.panel.baidu") },
    { value: "deepl", label: t("translate.panel.deepl") },
    { value: "youdao", label: t("translate.panel.youdao") },
    { value: "google", label: t("translate.panel.google") },
  ],
);

/** 按错误 kind 本地化提示，避免展示 Rust 中文 message。 */
function localizedError(e: unknown): string {
  const err = normalizeError(e);
  const key = `translate.error.${err.kind}`;
  return te(key) ? t(key) : t("translate.error.unknown");
}

onMounted(async (): Promise<void> => {
  try {
    const settings = await getTranslateSettings();
    enabled.value = settings.enabled;
    targetLang.value = settings.targetLang;
    provider.value = settings.provider;
    hasBuiltin.value = settings.hasBuiltinCredential;
    fingerprint.value = settings.customKeyFingerprint;
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
});

/** 保存配置；留空的凭据字段不传，避免清空已存 Key。 */
async function onSave(): Promise<void> {
  saving.value = true;
  try {
    await saveTranslateSettings({
      enabled: enabled.value,
      targetLang: targetLang.value,
      provider: provider.value,
      ...(customApiKey.value ? { customApiKey: customApiKey.value } : {}),
      ...(customAppId.value ? { customAppId: customAppId.value } : {}),
    });
    customApiKey.value = "";
    customAppId.value = "";
    const settings = await getTranslateSettings();
    fingerprint.value = settings.customKeyFingerprint;
    pushToast(t("translate.panel.saved"), "ok");
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <section class="translate-panel">
    <h2 class="translate-panel__title">{{ t("translate.panel.title") }}</h2>
    <p class="translate-panel__desc">{{ t("translate.panel.desc") }}</p>

    <div class="translate-panel__row">
      <span>
        <span class="translate-panel__name">{{ t("translate.panel.enabled") }}</span>
        <span class="translate-panel__hint">{{ t("translate.panel.enabledDesc") }}</span>
      </span>
      <AppToggle
        v-model="enabled"
        class="translate-panel__toggle"
        :aria-label="t('translate.panel.enabled')"
      />
    </div>

    <div class="translate-panel__row">
      <span class="translate-panel__name">{{ t("translate.panel.targetLang") }}</span>
      <select
        v-model="targetLang"
        class="translate-panel__select"
        :aria-label="t('translate.panel.targetLang')"
      >
        <option
          v-for="opt in langOptions"
          :key="opt.value"
          :value="opt.value"
        >
          {{ opt.label }}
        </option>
      </select>
    </div>

    <div class="translate-panel__row">
      <span class="translate-panel__name">{{ t("translate.panel.provider") }}</span>
      <select
        v-model="provider"
        class="translate-panel__select"
        :aria-label="t('translate.panel.provider')"
      >
        <option
          v-for="opt in providerOptions"
          :key="opt.value"
          :value="opt.value"
        >
          {{ opt.label }}
        </option>
      </select>
    </div>

    <p
      class="translate-panel__status"
      :class="{ 'is-missing': !hasBuiltin }"
    >
      {{
        hasBuiltin
          ? t("translate.panel.builtinReady")
          : t("translate.panel.builtinMissing")
      }}
    </p>
    <p
      v-if="!hasBuiltin && provider === 'baidu'"
      class="translate-panel__hint"
    >
      {{ t("translate.panel.builtinFallback") }}
    </p>

    <div class="translate-panel__row">
      <span class="translate-panel__name">{{ t("translate.panel.customKey") }}</span>
      <input
        v-model="customApiKey"
        class="translate-panel__input"
        type="password"
        autocomplete="off"
        :aria-label="t('translate.panel.customKey')"
      />
    </div>
    <div class="translate-panel__row">
      <span class="translate-panel__name">{{ t("translate.panel.customAppId") }}</span>
      <input
        v-model="customAppId"
        class="translate-panel__input"
        type="text"
        autocomplete="off"
        :aria-label="t('translate.panel.customAppId')"
      />
    </div>
    <p
      v-if="fingerprint"
      class="translate-panel__hint"
    >
      {{ t("translate.panel.keyConfigured", { fingerprint }) }}
    </p>
    <p class="translate-panel__hint">{{ t("translate.panel.keyHint") }}</p>

    <button
      class="translate-panel__save"
      type="button"
      :disabled="saving"
      @click="onSave"
    >
      {{ t("translate.panel.save") }}
    </button>
  </section>
</template>

<style scoped>
.translate-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 640px;
}

.translate-panel__title {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  color: var(--ink);
}

.translate-panel__desc,
.translate-panel__hint {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.5;
}

.translate-panel__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 12px 16px;
  border: 1px solid var(--line);
  border-radius: var(--r-md);
  background: var(--bg-soft);
}

/* 开关固定在右侧，描述文字较长时不被压缩 */
.translate-panel__toggle {
  flex-shrink: 0;
}

.translate-panel__name {
  display: block;
  color: var(--ink);
  font-size: 14px;
  font-weight: 600;
}

.translate-panel__hint {
  display: block;
  margin-top: 4px;
}

.translate-panel__select,
.translate-panel__input {
  min-width: 220px;
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: var(--r-sm);
  background: var(--bg-window);
  color: var(--ink);
  font-size: 13px;
}

.translate-panel__status {
  margin: 0;
  color: var(--matcha-deep);
  font-size: 12px;
}

.translate-panel__status.is-missing {
  color: var(--ink-soft);
}

.translate-panel__save {
  align-self: flex-start;
  padding: 8px 20px;
  border: 1px solid var(--matcha-deep);
  border-radius: var(--r-md);
  background: var(--bg-window);
  color: var(--ink);
  font-size: 13px;
  cursor: pointer;
}

.translate-panel__save:disabled {
  opacity: 0.6;
  cursor: wait;
}
</style>

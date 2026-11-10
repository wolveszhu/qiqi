<script setup lang="ts">
/**
 * 划词翻译设置面板：内置服务商（用量 / 额度 / 启停）、总开关、译入语言与多个自填服务商。
 *
 * 红线：明文 Key / APPID 只经 IPC 单向流向 Rust；读取只回脱敏指纹；
 * 所有 await IPC 都 try/catch 并按 kind 给本地化提示。
 * 布局：内置服务一张卡片，自填服务商一张卡片（列表顺序即优先级），基础设置一张卡片。
 */

import { computed, onMounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";

import { normalizeError } from "../../../api/client";
import {
  deleteCustomTranslateProvider,
  getTranslateSettings,
  listBuiltinTranslateProviders,
  listCustomTranslateProviders,
  moveCustomTranslateProvider,
  saveBuiltinTranslateProvider,
  saveCustomTranslateProvider,
  saveTranslateSettings,
} from "../../../api/translate";
import type {
  BuiltinTranslateProvider,
  CustomTranslateProvider,
  TranslateProviderId,
  TranslateTargetLang,
} from "../../../api/types";
import AppButton from "../../../components/ui/AppButton.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import AppInput from "../../../components/ui/AppInput.vue";
import AppToggle from "../../../components/ui/AppToggle.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import { pushToast } from "../../../composables/useToast";

const { t, te } = useI18n();

const enabled = ref(true);
const targetLang = ref<TranslateTargetLang>("auto");
const hasBuiltin = ref(false);
const saving = ref(false);

const builtins = ref<BuiltinTranslateProvider[]>([]);
const quotaDrafts = reactive<Record<number, string>>({});

const customs = ref<CustomTranslateProvider[]>([]);
const editorOpen = ref(false);
const editorSaving = ref(false);
const form = reactive({
  id: null as number | null,
  name: "",
  provider: "deepl" as TranslateProviderId,
  appId: "",
  apiKey: "",
  hasAppId: false,
  keyFingerprint: null as string | null,
  enabled: true,
});

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

function providerLabel(id: TranslateProviderId): string {
  return providerOptions.value.find((o) => o.value === id)?.label ?? id;
}

/** 当月用量百分比（夹到 100%，额度非正时退化 0）。 */
function usagePercent(p: BuiltinTranslateProvider): number {
  if (p.monthlyGranted <= 0) return 0;
  return Math.min(100, (p.used / p.monthlyGranted) * 100);
}

/** 内置服务名本地化（后端 name 同时是额度记账键，固定中文）。 */
function builtinLabel(p: BuiltinTranslateProvider): string {
  if (p.providerId === "xfyun") return t("translate.panel.builtinXfyun");
  if (p.providerId === "baidu") return t("translate.panel.builtinBaidu");
  return p.name;
}

/** 拉取两家内置服务商的当月用量与额度，初始化额度输入草稿。 */
async function refreshBuiltins(): Promise<void> {
  try {
    builtins.value = await listBuiltinTranslateProviders();
    for (const p of builtins.value) {
      quotaDrafts[p.id] = String(p.monthlyGranted);
    }
    hasBuiltin.value = builtins.value.some((p) => p.enabled && p.hasCredential);
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

/** 启停某家内置服务（额度沿用当前值）。 */
async function toggleBuiltin(p: BuiltinTranslateProvider, value: boolean): Promise<void> {
  try {
    await saveBuiltinTranslateProvider({
      id: p.id,
      enabled: value,
      monthlyGranted: p.monthlyGranted,
    });
    await refreshBuiltins();
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

/** 保存某家的月度上限；输入非法时给本地化提示而不发请求。 */
async function saveBuiltinQuota(p: BuiltinTranslateProvider): Promise<void> {
  const granted = Number(quotaDrafts[p.id]);
  if (!Number.isFinite(granted) || granted <= 0) {
    pushToast(t("translate.error.config"), "error");
    return;
  }
  try {
    await saveBuiltinTranslateProvider({
      id: p.id,
      enabled: p.enabled,
      monthlyGranted: Math.floor(granted),
    });
    await refreshBuiltins();
    pushToast(t("translate.panel.saved"), "ok");
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

/** 拉取自填服务商列表（列表顺序即优先级）。 */
async function refreshCustoms(): Promise<void> {
  try {
    customs.value = await listCustomTranslateProviders();
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

/** 保存总开关与译入语言（服务商选择由自填列表决定）。 */
async function onSave(): Promise<void> {
  saving.value = true;
  try {
    await saveTranslateSettings({
      enabled: enabled.value,
      targetLang: targetLang.value,
      provider: "baidu",
    });
    pushToast(t("translate.panel.saved"), "ok");
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  } finally {
    saving.value = false;
  }
}

/** 打开编辑器；传入已有项时为编辑，缺省为新增。 */
function openEditor(c?: CustomTranslateProvider): void {
  form.id = c?.id ?? null;
  form.name = c?.name ?? "";
  form.provider = c?.provider ?? "deepl";
  form.appId = "";
  form.apiKey = "";
  form.hasAppId = c?.hasAppId ?? false;
  form.keyFingerprint = c?.keyFingerprint ?? null;
  form.enabled = c?.enabled ?? true;
  editorOpen.value = true;
}

/** 新增 / 编辑：留空的凭据字段不传，避免清空已存值。 */
async function submitProvider(): Promise<void> {
  editorSaving.value = true;
  try {
    await saveCustomTranslateProvider({
      id: form.id ?? undefined,
      name: form.name,
      provider: form.provider,
      appId: form.appId || undefined,
      apiKey: form.apiKey || undefined,
      enabled: form.enabled,
    });
    editorOpen.value = false;
    await refreshCustoms();
    pushToast(t("translate.panel.saved"), "ok");
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  } finally {
    editorSaving.value = false;
  }
}

/** 启停复用保存命令（不传凭据字段，避免误清空）。 */
async function toggleProvider(c: CustomTranslateProvider, value: boolean): Promise<void> {
  try {
    await saveCustomTranslateProvider({
      id: c.id,
      name: c.name,
      provider: c.provider,
      enabled: value,
    });
    await refreshCustoms();
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

async function removeProvider(id: number): Promise<void> {
  try {
    await deleteCustomTranslateProvider(id);
    await refreshCustoms();
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

async function moveProvider(id: number, up: boolean): Promise<void> {
  try {
    await moveCustomTranslateProvider(id, up);
    await refreshCustoms();
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

onMounted(async (): Promise<void> => {
  try {
    const settings = await getTranslateSettings();
    enabled.value = settings.enabled;
    targetLang.value = settings.targetLang;
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
  await refreshBuiltins();
  await refreshCustoms();
});
</script>

<template>
  <section class="translate-panel">
    <h2 class="translate-panel__title">{{ t("translate.panel.title") }}</h2>
    <p class="translate-panel__desc">{{ t("translate.panel.desc") }}</p>

    <AppCard :title="t('translate.panel.builtinProviders')">
      <HintBox tone="info">{{ t("translate.panel.builtinHint") }}</HintBox>

      <ul class="builtin-list">
        <li
          v-for="p in builtins"
          :key="p.id"
          class="builtin-item"
          :class="{ 'is-off': !p.enabled }"
        >
          <div class="builtin-item__head">
            <div class="builtin-item__title">
              <span class="builtin-item__name">{{ builtinLabel(p) }}</span>
              <span
                v-if="p.used >= p.monthlyGranted"
                class="builtin-item__badge"
              >
                {{ t("translate.panel.builtinExhausted") }}
              </span>
            </div>
            <AppToggle
              :model-value="p.enabled"
              :aria-label="t('translate.panel.enableBuiltin')"
              @update:model-value="(v: boolean) => toggleBuiltin(p, v)"
            />
          </div>

          <div class="builtin-item__meter">
            <span
              class="builtin-item__fill"
              :style="{ width: `${usagePercent(p)}%` }"
            ></span>
          </div>

          <p class="builtin-item__meta">
            {{
              t("translate.panel.monthlyUsed", {
                used: p.used,
                granted: p.monthlyGranted,
                requests: p.requests,
              })
            }}
          </p>
          <p
            class="builtin-item__meta"
            :class="{ 'is-warn': !p.hasCredential }"
          >
            {{
              p.hasCredential
                ? t("translate.panel.builtinCredential", {
                    fp: p.credentialFingerprint ?? "",
                  })
                : t("translate.panel.builtinCredentialMissing")
            }}
          </p>

          <div class="builtin-item__quota">
            <label
              class="builtin-item__label"
              :for="`quota-${p.id}`"
            >
              {{ t("translate.panel.monthlyGranted") }}
            </label>
            <input
              :id="`quota-${p.id}`"
              v-model="quotaDrafts[p.id]"
              class="builtin-item__input"
              type="number"
              min="1"
              step="1000"
            />
            <AppButton
              variant="ghost"
              @click="saveBuiltinQuota(p)"
            >
              {{ t("translate.panel.saveBuiltin") }}
            </AppButton>
          </div>
        </li>
      </ul>
    </AppCard>

    <AppCard :title="t('translate.panel.customProviders')">
      <HintBox tone="info">{{ t("translate.panel.customHint") }}</HintBox>

      <ul
        v-if="customs.length"
        class="custom-list"
      >
        <li
          v-for="(c, i) in customs"
          :key="c.id"
          class="custom-item"
          :class="{ 'is-off': !c.enabled }"
        >
          <div class="custom-item__main">
            <span class="custom-item__name">{{ c.name }}</span>
            <span class="custom-item__meta">
              {{ t("translate.panel.priority", { n: i + 1 }) }} ·
              {{ providerLabel(c.provider) }}
            </span>
            <span
              class="custom-item__meta"
              :class="{ 'is-warn': !c.hasKey }"
            >
              {{
                c.hasKey
                  ? t("translate.panel.configuredKey", { fp: c.keyFingerprint ?? "" })
                  : t("translate.panel.unconfiguredKey")
              }}
            </span>
          </div>
          <div class="custom-item__side">
            <AppToggle
              :model-value="c.enabled"
              :aria-label="t('translate.panel.enableProvider')"
              @update:model-value="(v: boolean) => toggleProvider(c, v)"
            />
            <button
              class="custom-item__btn"
              type="button"
              :aria-label="t('translate.panel.moveUp')"
              :title="t('translate.panel.moveUp')"
              :disabled="i === 0"
              @click="moveProvider(c.id, true)"
            >
              ↑
            </button>
            <button
              class="custom-item__btn"
              type="button"
              :aria-label="t('translate.panel.moveDown')"
              :title="t('translate.panel.moveDown')"
              :disabled="i === customs.length - 1"
              @click="moveProvider(c.id, false)"
            >
              ↓
            </button>
            <button
              class="custom-item__btn"
              type="button"
              @click="openEditor(c)"
            >
              {{ t("common.edit") }}
            </button>
            <button
              class="custom-item__btn custom-item__btn--danger"
              type="button"
              :aria-label="t('translate.panel.deleteProvider')"
              :title="t('translate.panel.deleteProvider')"
              @click="removeProvider(c.id)"
            >
              ✕
            </button>
          </div>
        </li>
      </ul>
      <p
        v-else
        class="custom-empty"
      >
        {{ t("translate.panel.noProvider") }}
      </p>

      <AppButton
        variant="ghost"
        @click="openEditor()"
      >
        {{ t("translate.panel.addProvider") }}
      </AppButton>

      <div
        v-if="editorOpen"
        class="custom-editor"
      >
        <AppInput
          v-model="form.name"
          :label="t('translate.panel.providerName')"
          :placeholder="t('translate.panel.providerNamePlaceholder')"
          :maxlength="20"
        />
        <div class="custom-editor__row">
          <span class="custom-editor__label">{{ t("translate.panel.serviceType") }}</span>
          <select
            v-model="form.provider"
            class="translate-panel__select"
            :aria-label="t('translate.panel.serviceType')"
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
        <AppInput
          v-model="form.appId"
          :label="t('translate.panel.appId')"
          :placeholder="form.hasAppId ? t('translate.panel.appIdHint') : ''"
        />
        <p class="custom-editor__hint">{{ t("translate.panel.appIdHint") }}</p>
        <AppInput
          v-model="form.apiKey"
          type="password"
          :label="t('translate.panel.apiKey')"
          :placeholder="
            form.keyFingerprint
              ? t('translate.panel.apiKeyHas', { fp: form.keyFingerprint })
              : t('translate.panel.apiKeyPlaceholder')
          "
        />
        <div class="custom-editor__toggle">
          <AppToggle
            :model-value="form.enabled"
            :label="t('translate.panel.enableProvider')"
            @update:model-value="(v: boolean) => (form.enabled = v)"
          />
        </div>
        <div class="custom-editor__actions">
          <AppButton
            variant="primary"
            :loading="editorSaving"
            @click="submitProvider"
          >
            {{ t("translate.panel.saveProvider") }}
          </AppButton>
          <AppButton
            variant="ghost"
            @click="editorOpen = false"
          >
            {{ t("common.cancel") }}
          </AppButton>
        </div>
      </div>
    </AppCard>

    <AppCard :title="t('translate.panel.title')">
      <div class="translate-panel__row">
        <span>
          <span class="translate-panel__name">{{ t("translate.panel.enabled") }}</span>
          <span class="translate-panel__hint">{{
            t("translate.panel.enabledDesc")
          }}</span>
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

      <AppButton
        variant="primary"
        :loading="saving"
        @click="onSave"
      >
        {{ t("translate.panel.save") }}
      </AppButton>
    </AppCard>
  </section>
</template>

<style scoped>
.translate-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 720px;
}

.translate-panel__title {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  color: var(--ink);
}

.translate-panel__desc,
.translate-panel__hint,
.custom-editor__hint {
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

.translate-panel__select {
  min-width: 200px;
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

.custom-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.custom-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: var(--r-md);
  background: var(--bg-window);
  transition: opacity var(--dur-fast) var(--ease-jelly);
}

.custom-item.is-off {
  opacity: 0.55;
}

.custom-item__main {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.custom-item__name {
  color: var(--ink);
  font-size: 14px;
  font-weight: 600;
  overflow-wrap: anywhere;
}

.custom-item__meta {
  color: var(--ink-soft);
  font-size: 12px;
}

.custom-item__meta.is-warn {
  color: var(--peach);
}

.custom-item__side {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.custom-item__btn {
  min-width: 26px;
  height: 26px;
  padding: 0 6px;
  border: 1px solid var(--line);
  border-radius: var(--r-sm);
  background: transparent;
  color: var(--ink);
  font-size: 12px;
  cursor: pointer;
}

.custom-item__btn:hover:not(:disabled) {
  border-color: var(--peach);
}

.custom-item__btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.custom-item__btn--danger {
  color: var(--ink-soft);
}

.custom-empty {
  margin: 0;
  padding: 14px;
  border: 1px dashed var(--line);
  border-radius: var(--r-md);
  color: var(--ink-soft);
  font-size: 13px;
  text-align: center;
}

.custom-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 12px;
  padding: 14px;
  border: 1px dashed var(--line);
  border-radius: var(--r-md);
  background: var(--bg-soft);
}

.custom-editor__row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.custom-editor__label {
  color: var(--ink);
  font-size: 13px;
  font-weight: 600;
}

.custom-editor__toggle {
  align-self: flex-start;
}

.custom-editor__actions {
  display: flex;
  gap: 8px;
}

/* 内置服务商：用量条 + 各自额度编辑 */
.builtin-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.builtin-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: var(--r-md);
  background: var(--bg-soft);
  transition: opacity var(--dur-fast) var(--ease-jelly);
}

.builtin-item.is-off {
  opacity: 0.55;
}

.builtin-item__head,
.builtin-item__title,
.builtin-item__quota {
  display: flex;
  align-items: center;
  gap: 10px;
}

.builtin-item__head {
  justify-content: space-between;
}

.builtin-item__name {
  color: var(--ink);
  font-size: 14px;
  font-weight: 600;
}

.builtin-item__badge {
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--peach);
  color: var(--ink);
  font-size: 11px;
}

.builtin-item__meter {
  height: 6px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--line);
}

.builtin-item__fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--matcha-deep);
  transition: width var(--dur-fast) var(--ease-jelly);
}

.builtin-item__meta {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
}

.builtin-item__meta.is-warn {
  color: var(--peach);
}

.builtin-item__quota {
  flex-wrap: wrap;
}

.builtin-item__label {
  color: var(--ink);
  font-size: 12px;
  font-weight: 600;
}

.builtin-item__input {
  width: 120px;
  padding: 6px 9px;
  border: 1px solid var(--line);
  border-radius: var(--r-sm);
  background: var(--bg-window);
  color: var(--ink);
  font-size: 13px;
}
</style>

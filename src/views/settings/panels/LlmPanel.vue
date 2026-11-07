<script setup lang="ts">
/**
 * 大模型面板（用户决策 2026-10-24：三档配置与自定义多模型**合并为单一服务商管理**）。
 *
 * - 平台预设来自 Rust `PROVIDER_PRESETS` 单一真源（9 家主流平台），点击即预填
 *   名称 / Base URL / 常见模型；URL 可改、模型可自填；
 * - 平台列表顺序 = 编排优先级：自定义（顺序 failover）→ 内置免费模型 → 本地话术；
 * - 模型用 ModelChips（回车/逗号添加为标签）；API Key **必须手动输入**；
 * - 记忆沉淀档由后端自动派生（取首个启用平台），UI 不再暴露。
 */
import { computed, onMounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";

import AppButton from "../../../components/ui/AppButton.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import AppInput from "../../../components/ui/AppInput.vue";
import AppToggle from "../../../components/ui/AppToggle.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import StatusBadge from "../../../components/ui/StatusBadge.vue";
import ModelChips from "../../../components/settings/ModelChips.vue";
import ProviderChip from "../../../components/settings/ProviderChip.vue";
import QuotaBar from "../../../components/settings/QuotaBar.vue";
import { normalizeError } from "../../../api/client";
import {
  deleteCustomModel,
  getSearchSettings,
  getTyped,
  listCustomModels,
  listProviders,
  moveCustomModel,
  saveCustomModel,
  saveSearchSettings,
  setQuotaMode,
} from "../../../api/settings";
import { getQuotaSummary, listQuotaRecords } from "../../../api/quota";
import { pushToast } from "../../../composables/useToast";
import { setGlobalDefault } from "../../../composables/useWebSearchPreference";
import { applySnapshot, settingsStore } from "../../../state/settingsStore";
import type {
  CustomModelDto,
  ProviderPreset,
  QuotaSummary,
  QuotaUsageRecord,
  SaveCustomModelPayload,
  SaveSearchSettings,
  SearchSettings,
} from "../../../api/types";

const { t } = useI18n();

/** 服务商预设（Rust 单一真源经 IPC 下发，前端不复制）。 */
const providers = ref<ProviderPreset[]>([]);
/** 已配置的自定义平台（顺序即编排优先级）。 */
const customs = ref<CustomModelDto[]>([]);

const customEditorOpen = ref(false);
const customSaving = ref(false);
const customForm = reactive({
  id: null as number | null,
  /** 编辑器对应的预设 id（chips 高亮 + 模型建议源；纯自定义平台为 null）。 */
  presetId: null as string | null,
  name: "",
  baseUrl: "",
  models: [] as string[],
  /** 用户新输入的 Key（空=不改动存量）。 */
  keyInput: "",
  keyHas: false,
  keyFingerprint: null as string | null,
  enabled: true,
});

/** 联网搜索服务配置（BYOK；默认关，未配置 Key 时后端自动回落普通对话）。 */
const searchSaving = ref(false);
const searchForm = reactive({
  provider: "tavily",
  apiKey: "",
  hasKey: false,
  keyFingerprint: null as string | null,
  enabled: false,
  maxResults: 5,
  lang: "zh",
});
/** 可选搜索服务商（与 Rust `SearchProvider` 对齐；前端仅展示，真源在后端）。 */
const searchProviders = computed<Array<{ id: string; label: string }>>(() => [
  { id: "tavily", label: t("llm.providerTavily") },
  { id: "brave", label: t("llm.providerBrave") },
  { id: "serpapi", label: t("llm.providerSerpapi") },
]);

/** 刷新自定义平台列表（失败 toast——不让用户以为保存成功）。 */
async function refreshCustoms(): Promise<void> {
  try {
    customs.value = await listCustomModels();
  } catch (e) {
    pushToast(normalizeError(e).message, "error");
  }
}

/** 打开编辑器（带参=编辑现有平台，无参=空白新增）。 */
function openCustomEditor(item?: CustomModelDto): void {
  customEditorOpen.value = true;
  customForm.id = item?.id ?? null;
  customForm.presetId = null;
  customForm.name = item?.name ?? "";
  customForm.baseUrl = item?.baseUrl ?? "";
  customForm.models = item ? [...item.models] : [];
  customForm.keyInput = "";
  customForm.keyHas = item?.hasKey ?? false;
  customForm.keyFingerprint = item?.keyFingerprint ?? null;
  customForm.enabled = item?.enabled ?? true;
}

/** 点击平台预设 → 预填编辑器（名称/URL 预填；**模型不默认选中**，由用户在建议里点选或手输；Key 永远手输）。 */
function openPresetEditor(preset: ProviderPreset): void {
  openCustomEditor();
  customForm.presetId = preset.id;
  customForm.name = preset.label;
  customForm.baseUrl = preset.baseUrl;
  customForm.models = [];
}

/** 保存平台（新建或更新）。 */
async function submitCustom(): Promise<void> {
  const payload: SaveCustomModelPayload = {
    id: customForm.id ?? undefined,
    name: customForm.name.trim(),
    baseUrl: customForm.baseUrl.trim(),
    models: customForm.models,
    // 留空 = 不改动存量 Key（新建时即暂不配置，由 UI 提示补配）
    apiKey: customForm.keyInput || undefined,
    enabled: customForm.enabled,
  };
  customSaving.value = true;
  try {
    await saveCustomModel(payload);
    await refreshCustoms();
    customEditorOpen.value = false;
    pushToast(t("common.saved"));
  } catch (e) {
    pushToast(normalizeError(e).message, "error");
  } finally {
    customSaving.value = false;
  }
}

/** 删除平台。 */
async function removeCustom(id: number): Promise<void> {
  try {
    await deleteCustomModel(id);
    await refreshCustoms();
    pushToast(t("llm.deleted"));
  } catch (e) {
    pushToast(normalizeError(e).message, "error");
  }
}

/** 上移/下移一位（调整编排优先级；越界时 Rust 静默成功）。 */
async function moveCustom(id: number, up: boolean): Promise<void> {
  try {
    await moveCustomModel(id, up);
    await refreshCustoms();
  } catch (e) {
    pushToast(normalizeError(e).message, "error");
  }
}

/** 读取联网搜索配置（填充表单；失败仅 console，不阻断主流程）。 */
async function refreshSearch(): Promise<void> {
  try {
    const s: SearchSettings = await getSearchSettings();
    searchForm.provider = s.provider;
    searchForm.hasKey = s.hasKey;
    searchForm.keyFingerprint = s.keyFingerprint;
    searchForm.enabled = s.enabled;
    searchForm.maxResults = s.maxResults;
    searchForm.lang = s.lang;
    searchForm.apiKey = "";
    // 同步全局默认到共享状态（保存成功后经此刷新）→ 对话框开关跟随
    setGlobalDefault(s.enabled);
  } catch (e) {
    console.warn("[llm] 联网搜索配置读取失败", e);
  }
}

/** 保存联网搜索配置（apiKey 留空 = 不改动存量 Key）。 */
async function saveSearch(): Promise<void> {
  const payload: SaveSearchSettings = {
    provider: searchForm.provider,
    apiKey: searchForm.apiKey || undefined,
    enabled: searchForm.enabled,
    maxResults: searchForm.maxResults,
    lang: searchForm.lang,
  };
  searchSaving.value = true;
  try {
    await saveSearchSettings(payload);
    await refreshSearch();
    pushToast(t("common.saved"));
  } catch (e) {
    pushToast(normalizeError(e).message, "error");
  } finally {
    searchSaving.value = false;
  }
}

/** 启停切换（保存入口；apiKey 缺省 = 不动存量 Key）。 */
async function toggleCustomEnabled(
  item: CustomModelDto,
  enabled: boolean,
): Promise<void> {
  try {
    await saveCustomModel({
      id: item.id,
      name: item.name,
      baseUrl: item.baseUrl,
      models: item.models,
      enabled,
    });
    await refreshCustoms();
  } catch (e) {
    pushToast(normalizeError(e).message, "error");
  }
}

async function toggleQuota(useBuiltin: boolean): Promise<void> {
  try {
    await setQuotaMode(useBuiltin ? 1 : 2);
    const snap = await getTyped();
    applySnapshot(snap);
    pushToast(useBuiltin ? t("llm.switchedBuiltin") : t("llm.switchedOwn"));
    await refreshQuota();
  } catch (e) {
    pushToast(normalizeError(e).message, "error");
  }
}

/** 内置体验额度：当月汇总 + 明细（仅内置额度记账；BYOK 不展示）。 */
const quotaSummary = ref<QuotaSummary | null>(null);
const quotaRecords = ref<QuotaUsageRecord[]>([]);
const quotaDetailsOpen = ref(false);
const quotaLoading = ref(false);

/** 场景标签（与 Rust scene 常量对齐；缺省回落原值）。 */
function sceneLabel(scene: string): string {
  const map: Record<string, string> = {
    chat: t("llm.sceneChat"),
    proactive: t("llm.sceneProactive"),
    distill: t("llm.sceneDistill"),
    diary: t("llm.sceneDiary"),
  };
  return map[scene] ?? scene;
}

/** credits 数字千分位（等宽对齐扫读）。 */
function fmtCredits(n: number): string {
  return n.toLocaleString("zh-CN");
}

/** 徽章状态：BYOK → warn；耗尽 → warn；低余量（<20%）→ warn；否则 ok。 */
const quotaTone = computed<"ok" | "warn">(() => {
  if (settingsStore.snapshot?.isCustomQuotaUsed === 2) return "warn";
  if (!quotaSummary.value) return "ok";
  return quotaSummary.value.remainingRatio <= 0.2 ? "warn" : "ok";
});

const quotaBadgeText = computed(() => {
  if (settingsStore.snapshot?.isCustomQuotaUsed === 2) return t("llm.useOwnKey");
  if (quotaSummary.value && quotaSummary.value.remainingRatio <= 0)
    return t("llm.quotaExhaustedBadge");
  return t("llm.builtinEnabledBadge");
});

/** 拉取汇总与明细（失败 toast——不让用户以为加载成功）。 */
async function refreshQuota(): Promise<void> {
  quotaLoading.value = true;
  try {
    const [summary, records] = await Promise.all([getQuotaSummary(), listQuotaRecords()]);
    quotaSummary.value = summary;
    quotaRecords.value = records;
  } catch (e) {
    pushToast(e instanceof Error ? e.message : t("llm.quotaLoadFailed"), "error");
  } finally {
    quotaLoading.value = false;
  }
}

onMounted(async (): Promise<void> => {
  // 四路独立加载并行拉取（预设/平台/额度/联网配置），缩短面板就绪时间；
  // 各成员内部均已兜错（toast/console.warn，不会 reject），Promise.all 无需再包 catch
  await Promise.all([
    listProviders()
      .then((list) => (providers.value = list))
      .catch((e: unknown) => console.warn("[llm] 服务商列表拉取失败", e)),
    refreshCustoms(),
    refreshQuota(),
    refreshSearch(),
  ]);
});
</script>

<template>
  <div class="llm-panel">
    <!-- 卡片 1：自定义大模型服务商（平台预设 + 平台列表 + 编辑器） -->
    <AppCard :title="t('llm.customProvider')">
      <HintBox tone="info">
        {{ t("llm.customHint") }}
      </HintBox>

      <!-- 平台预设：点击即预填（URL 可改、模型按需点选、Key 手输）；手动新增入口跟随其后 -->
      <div class="preset-block">
        <span class="preset-block__label">{{ t("llm.pickPlatform") }}</span>
        <div class="preset-row">
          <ProviderChip
            v-for="p in providers"
            :key="p.id"
            :label="p.label"
            :required="p.required"
            :cost-tier="p.costTier"
            :selected="customForm.presetId === p.id"
            :quota-candidate="p.costTier.includes('免费')"
            @select="openPresetEditor(p)"
          />
          <AppButton
            variant="ghost"
            class="preset-row__add"
            @click="openCustomEditor()"
          >
            {{ t("llm.manualAdd") }}
          </AppButton>
        </div>
      </div>

      <!-- 已配置平台列表 -->
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
            <span class="custom-item__models">{{
              c.models.length ? c.models.join(" · ") : t("llm.noModels")
            }}</span>
            <span class="custom-item__url">{{ c.baseUrl }}</span>
          </div>
          <div class="custom-item__side">
            <StatusBadge
              :tone="c.hasKey ? 'ok' : 'warn'"
              :text="
                c.hasKey
                  ? t('llm.configuredKey', { fp: c.keyFingerprint ?? '' })
                  : t('llm.unconfiguredKey')
              "
            />
            <AppToggle
              :model-value="c.enabled"
              @update:model-value="(v: boolean) => toggleCustomEnabled(c, v)"
            />
            <button
              class="custom-item__btn"
              type="button"
              :aria-label="t('llm.moveUp')"
              :disabled="i === 0"
              @click="moveCustom(c.id, true)"
            >
              ↑
            </button>
            <button
              class="custom-item__btn"
              type="button"
              :aria-label="t('llm.moveDown')"
              :disabled="i === customs.length - 1"
              @click="moveCustom(c.id, false)"
            >
              ↓
            </button>
            <button
              class="custom-item__btn"
              type="button"
              @click="openCustomEditor(c)"
            >
              {{ t("common.edit") }}
            </button>
            <button
              class="custom-item__btn custom-item__btn--danger"
              type="button"
              :aria-label="t('llm.deletePlatform')"
              @click="removeCustom(c.id)"
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
        {{ t("llm.noPlatform") }}
      </p>

      <!-- 编辑器 -->
      <div
        v-if="customEditorOpen"
        class="custom-editor"
      >
        <AppInput
          v-model="customForm.name"
          :label="t('llm.name')"
          :placeholder="t('llm.namePlaceholder')"
          :maxlength="20"
          :disabled="customForm.presetId !== null"
        />
        <p
          v-if="customForm.presetId !== null"
          class="custom-editor__hint"
        >
          {{ t("llm.presetNameHint") }}
        </p>
        <AppInput
          v-model="customForm.baseUrl"
          :label="t('llm.baseUrl')"
          :placeholder="t('llm.baseUrlPlaceholder')"
        />
        <div class="custom-editor__models">
          <span class="custom-editor__label">{{ t("llm.models") }}</span>
          <ModelChips
            :model-value="customForm.models"
            :suggestions="
              providers.find((p) => p.id === customForm.presetId)?.defaultModels ?? []
            "
            @update:model-value="(v: string[]) => (customForm.models = v)"
          />
          <p
            v-if="!customForm.models.length"
            class="custom-editor__hint"
          >
            {{ t("llm.modelsHint") }}
          </p>
        </div>
        <AppInput
          v-model="customForm.keyInput"
          type="password"
          :label="t('llm.apiKey')"
          :placeholder="
            customForm.keyHas
              ? t('llm.apiKeyHas', { fp: customForm.keyFingerprint ?? '' })
              : t('llm.apiKeyPlaceholder')
          "
        />
        <!-- align-self 收缩包裹：label 默认会随列向 flex 拉伸成整行，点击范围随之扩大到整行 -->
        <div class="custom-editor__toggle">
          <AppToggle
            :model-value="customForm.enabled"
            :label="t('llm.enablePlatform')"
            @update:model-value="(v: boolean) => (customForm.enabled = v)"
          />
        </div>
        <div class="custom-editor__actions">
          <AppButton
            variant="primary"
            :loading="customSaving"
            @click="submitCustom"
          >
            {{ t("llm.save") }}
          </AppButton>
          <AppButton
            variant="ghost"
            @click="customEditorOpen = false"
          >
            {{ t("common.cancel") }}
          </AppButton>
        </div>
      </div>

      <HintBox tone="info">
        {{ t("llm.billingNote") }}
      </HintBox>
    </AppCard>

    <!-- 卡片 2：内置体验额度 -->
    <AppCard :title="t('llm.builtinQuota')">
      <div class="llm-panel__quota">
        <div class="llm-panel__quota-head">
          <span>{{ t("llm.remainingQuota") }}</span>
          <StatusBadge
            :tone="quotaTone"
            :text="quotaBadgeText"
          />
        </div>
        <QuotaBar :ratio="quotaSummary?.remainingRatio ?? 1" />
        <div
          v-if="quotaSummary"
          class="quota-summary"
          :aria-label="t('llm.quotaSummaryAria')"
        >
          <div class="quota-summary__item">
            <span class="quota-summary__num">{{ fmtCredits(quotaSummary.used) }}</span>
            <span class="quota-summary__label">{{ t("llm.usedThisMonth") }}</span>
          </div>
          <div class="quota-summary__item">
            <span class="quota-summary__num">{{ fmtCredits(quotaSummary.granted) }}</span>
            <span class="quota-summary__label">{{ t("llm.monthlyTotal") }}</span>
          </div>
          <div class="quota-summary__item">
            <span class="quota-summary__num">{{ quotaSummary.requests }}</span>
            <span class="quota-summary__label">{{ t("llm.requests") }}</span>
          </div>
          <div class="quota-summary__item">
            <span class="quota-summary__num">{{ quotaSummary.periodKey }}</span>
            <span class="quota-summary__label">{{ t("llm.billingCycle") }}</span>
          </div>
          <AppButton
            variant="ghost"
            class="quota-summary__details-btn"
            @click="quotaDetailsOpen = !quotaDetailsOpen"
          >
            {{ quotaDetailsOpen ? t("llm.hideDetails") : t("llm.viewDetails") }}
          </AppButton>
        </div>
        <!-- 按平台拆分的当月用量（仅内置额度段记账） -->
        <div
          v-if="quotaSummary?.platforms?.length"
          class="quota-platforms"
          :aria-label="t('llm.platformUsageAria')"
        >
          <span class="quota-platforms__title">{{ t("llm.platformUsage") }}</span>
          <ul class="quota-platforms__list">
            <li
              v-for="p in quotaSummary.platforms"
              :key="p.platform"
              class="quota-platforms__row"
            >
              <span class="quota-platforms__name">{{ p.platform }}</span>
              <span class="quota-platforms__used"
                >{{ fmtCredits(p.used) }} 点 · {{ p.requests }} 次</span
              >
            </li>
          </ul>
        </div>
        <p
          v-if="quotaSummary && quotaSummary.remainingRatio <= 0"
          class="llm-panel__quota-note llm-panel__quota-note--warn"
        >
          {{ t("llm.quotaExhausted") }}
        </p>
        <p
          v-else
          class="llm-panel__quota-note"
        >
          {{ t("llm.quotaMonthly") }}
        </p>

        <!-- 明细抽屉（时间倒序；空状态说明） -->
        <div
          v-if="quotaDetailsOpen"
          class="quota-details"
        >
          <p
            v-if="quotaRecords.length === 0"
            class="quota-details__empty"
          >
            {{ t("llm.quotaEmpty") }}
          </p>
          <ul
            v-else
            class="quota-details__list"
          >
            <li
              v-for="r in quotaRecords"
              :key="r.id"
              class="quota-details__row"
              :class="{ 'quota-details__row--fail': !r.ok }"
            >
              <span class="quota-details__time">{{ r.createdAt.slice(5, 16) }}</span>
              <span class="quota-details__scene">{{ sceneLabel(r.scene) }}</span>
              <span class="quota-details__platform">{{ r.platform }}</span>
              <span class="quota-details__model">{{ r.model }}</span>
              <span class="quota-details__tokens"
                >{{ r.promptTokens }}/{{ r.completionTokens }}</span
              >
              <span class="quota-details__credits">-{{ fmtCredits(r.credits) }}</span>
            </li>
          </ul>
        </div>
      </div>
      <AppToggle
        :model-value="settingsStore.snapshot?.isCustomQuotaUsed !== 2"
        :label="t('llm.useBuiltin')"
        @update:model-value="toggleQuota($event)"
      />
    </AppCard>

    <!-- 卡片 3：联网搜索（实时资料，BYOK） -->
    <AppCard :title="t('llm.webSearch')">
      <HintBox tone="info">
        {{ t("llm.webSearchHint") }}
      </HintBox>
      <div class="search-form">
        <label class="search-form__label">{{ t("llm.searchProvider") }}</label>
        <select
          v-model="searchForm.provider"
          class="search-form__select"
        >
          <option
            v-for="p in searchProviders"
            :key="p.id"
            :value="p.id"
          >
            {{ p.label }}
          </option>
        </select>

        <AppInput
          v-model="searchForm.apiKey"
          type="password"
          :label="t('llm.apiKey')"
          :placeholder="
            searchForm.hasKey
              ? t('llm.apiKeyHas', { fp: searchForm.keyFingerprint ?? '' })
              : t('llm.apiKeyPlaceholder')
          "
        />

        <div class="search-form__row">
          <label class="search-form__label">{{ t("llm.maxResults") }}</label>
          <input
            v-model.number="searchForm.maxResults"
            class="search-form__num"
            type="number"
            min="1"
            max="10"
          />
        </div>

        <div class="search-form__row">
          <label class="search-form__label">{{ t("llm.resultLang") }}</label>
          <select
            v-model="searchForm.lang"
            class="search-form__select search-form__select--sm"
          >
            <option value="zh">{{ t("llm.langZh") }}</option>
            <option value="en">{{ t("llm.langEn") }}</option>
          </select>
        </div>

        <div class="search-form__toggle">
          <AppToggle
            :model-value="searchForm.enabled"
            :label="t('llm.enableWebSearch')"
            @update:model-value="(v: boolean) => (searchForm.enabled = v)"
          />
        </div>
      </div>
      <div class="search-form__actions">
        <AppButton
          variant="primary"
          :loading="searchSaving"
          @click="saveSearch"
        >
          {{ t("llm.save") }}
        </AppButton>
      </div>
    </AppCard>
  </div>
</template>

<style scoped>
.llm-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
/* ---- 平台预设 ---- */
.preset-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 4px;
}
.preset-block__label {
  font-size: 12px;
  font-weight: 700;
  color: var(--ink-soft);
}
.preset-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
/* 手动新增入口：缩小到与 chip 同量级，作为预设行最后一项 */
.preset-row__add {
  padding: 6px 12px;
  font-size: 12px;
  border: 1px dashed var(--line);
  border-radius: 999px;
  color: var(--ink-soft);
}
.preset-row__add:hover {
  color: var(--matcha-deep);
  border-color: var(--matcha);
}
/* ---- 自定义平台列表 ---- */
.custom-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 12px 0;
}
.custom-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 12px;
  background: var(--bg-soft);
  border: 1px solid var(--line);
}
.custom-item.is-off {
  opacity: 0.55;
}
.custom-item__main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.custom-item__name {
  font-size: 14px;
  font-weight: 700;
  color: var(--ink);
}
.custom-item__models {
  font-size: 12px;
  color: var(--ink-soft);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.custom-item__url {
  font-size: 11px;
  color: var(--ink-soft);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.custom-item__side {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.custom-item__btn {
  border: none;
  background: var(--bg-window);
  color: var(--ink-soft);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 12px;
  cursor: pointer;
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
}
.custom-item__btn:hover:not(:disabled) {
  background: var(--matcha-soft, var(--bg-soft));
  color: var(--matcha-deep);
}
.custom-item__btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.custom-item__btn--danger:hover {
  background: var(--peach-soft);
  color: var(--peach);
}
.custom-empty {
  font-size: 13px;
  color: var(--ink-soft);
  margin: 12px 0;
}
/* 开关收缩包裹：列向 flex 会把子项拉伸成整行，导致 label 点击范围扩大到整行 */
.custom-editor__toggle {
  align-self: flex-start;
}
/* ---- 编辑器 ---- */
.custom-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 12px;
  padding: 14px;
  border-radius: 12px;
  background: var(--bg-soft);
  border: 1px dashed var(--line);
}
.custom-editor__models {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.custom-editor__label {
  font-size: 12px;
  font-weight: 600;
  color: var(--ink-soft);
}
.custom-editor__hint {
  font-size: 11px;
  color: var(--ink-soft);
  margin: 0;
}
.custom-editor__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
/* ---- 内置额度卡片 ---- */
.llm-panel__quota {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 12px;
}
.llm-panel__quota-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 14px;
  color: var(--ink);
}
.llm-panel__quota-note {
  font-size: 12px;
  color: var(--ink-soft);
  margin: 0;
}
.llm-panel__quota-note--warn {
  color: var(--peach);
}
/* ---- 用量汇总区（等宽数字对齐，便于扫读） ---- */
.quota-summary {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 12px;
  border-radius: 12px;
  background: var(--bg-deep);
}
.quota-summary__item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.quota-summary__num {
  font-size: 13px;
  font-weight: 700;
  color: var(--ink);
  font-variant-numeric: tabular-nums;
}
.quota-summary__label {
  font-size: 11px;
  color: var(--ink-soft);
}
.quota-summary__details-btn {
  margin-left: auto;
}
/* ---- 按平台用量（仅内置额度段） ---- */
.quota-platforms {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 8px 12px;
  border-radius: 12px;
  background: var(--bg-soft);
}
.quota-platforms__title {
  flex: none;
  font-size: 11px;
  color: var(--ink-soft);
  padding-top: 2px;
}
.quota-platforms__list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 14px;
  list-style: none;
  margin: 0;
  padding: 0;
}
.quota-platforms__row {
  display: flex;
  align-items: baseline;
  gap: 6px;
  font-size: 11px;
}
.quota-platforms__name {
  color: var(--ink);
  font-weight: 600;
}
.quota-platforms__used {
  color: var(--ink-soft);
  font-variant-numeric: tabular-nums;
}
/* ---- 明细抽屉（时间倒序；滚动区隐藏滚动条，与窗口 B 既有风格一致） ---- */
.quota-details {
  max-height: 240px;
  overflow-y: auto;
  scrollbar-width: none;
  border-radius: 12px;
  background: var(--bg-deep);
  padding: 4px 12px;
}
.quota-details::-webkit-scrollbar {
  display: none;
}
.quota-details__empty {
  font-size: 12px;
  color: var(--ink-soft);
  text-align: center;
  padding: 16px 0;
  margin: 0;
}
.quota-details__list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.quota-details__row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 0;
  font-size: 11px;
  color: var(--ink-soft);
  border-bottom: 1px solid var(--line);
}
.quota-details__row:last-child {
  border-bottom: none;
}
.quota-details__row--fail .quota-details__credits {
  color: var(--peach);
}
.quota-details__time {
  font-variant-numeric: tabular-nums;
  min-width: 84px;
}
.quota-details__scene {
  min-width: 56px;
  color: var(--ink);
}
.quota-details__platform {
  min-width: 64px;
  color: var(--matcha-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.quota-details__model {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.quota-details__tokens {
  font-variant-numeric: tabular-nums;
  min-width: 72px;
  text-align: right;
}
.quota-details__credits {
  font-variant-numeric: tabular-nums;
  min-width: 56px;
  text-align: right;
  font-weight: 700;
  color: var(--ink);
}
/* ---- 联网搜索卡片 ---- */
.search-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 12px 0;
}
.search-form__label {
  font-size: 12px;
  font-weight: 600;
  color: var(--ink-soft);
}
.search-form__select {
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--bg-window);
  color: var(--ink);
  padding: 7px 10px;
  font: 500 13px var(--font-body);
  outline: none;
}
.search-form__select:focus {
  border-color: var(--matcha);
}
.search-form__select--sm {
  width: 120px;
}
.search-form__num {
  width: 96px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--bg-window);
  color: var(--ink);
  padding: 7px 10px;
  font: 500 13px var(--font-body);
  outline: none;
}
.search-form__num:focus {
  border-color: var(--matcha);
}
.search-form__row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.search-form__toggle {
  align-self: flex-start;
}
.search-form__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>

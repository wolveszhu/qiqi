<script setup lang="ts">
/**
 * 大模型面板（设计方案 §4.2 + 阶段3补充选型方案，勘误 #16）：
 * 三档 SegmentedControl（main/memory/fallback）→ 每档独立表单（ProviderChip 组 + Base URL/模型/Key）；
 * 必选约束引导、fallback 回落提示、计费说明入口、成本档/额度池候选角标；显式保存 → settings_save_typed → toast + 快照刷新。
 * 卡片2：内置额度 toggle（setQuotaMode）+ QuotaBar 占位（体验额度）。
 */
import { onMounted, reactive, ref } from "vue";

import AppButton from "../../../components/ui/AppButton.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import AppInput from "../../../components/ui/AppInput.vue";
import AppToggle from "../../../components/ui/AppToggle.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import SegmentedControl from "../../../components/ui/SegmentedControl.vue";
import StatusBadge from "../../../components/ui/StatusBadge.vue";
import ProviderChip from "../../../components/settings/ProviderChip.vue";
import QuotaBar from "../../../components/settings/QuotaBar.vue";
import { getTyped, listProviders, saveTyped, setQuotaMode } from "../../../api/settings";
import { pushToast } from "../../../composables/useToast";
import { applySnapshot, settingsStore } from "../../../state/settingsStore";
import type {
  LlmTierConfig,
  ProviderPreset,
  SaveTierPayload,
  TierName,
} from "../../../api/types";

interface TierForm {
  baseUrl: string;
  model: string;
  modelCustom: string;
  /** 用户新输入的 Key（空=不改动存量）。 */
  keyInput: string;
  /** 脱敏指纹（回显，绝不明文）。 */
  keyFingerprint: string | null;
  keyHas: boolean;
}

const TIERS: Array<{ value: TierName; label: string; sub: string }> = [
  { value: "main", label: "主对话", sub: "main" },
  { value: "memory", label: "记忆沉淀", sub: "memory" },
  { value: "fallback", label: "兜底", sub: "fallback" },
];

/** 内置额度池候选（呼应 C3 EMBEDDED_QUOTA_BASE_URL；展示角标用，不依赖精确 id）。 */
const QUOTA_CANDIDATE_IDS = new Set([
  "builtin",
  "builtin-quota",
  "siliconflow",
  "glm-flash",
]);

const activeTier = ref<TierName>("main");
const providers = ref<ProviderPreset[]>([]);
const selected = reactive<Record<TierName, string | null>>({
  main: null,
  memory: null,
  fallback: null,
});
const forms = reactive<Record<TierName, TierForm>>({
  main: blankForm(),
  memory: blankForm(),
  fallback: blankForm(),
});
const saving = ref(false);
const initialized = ref(false);

function blankForm(): TierForm {
  return {
    baseUrl: "",
    model: "",
    modelCustom: "",
    keyInput: "",
    keyFingerprint: null,
    keyHas: false,
  };
}

function tierConfig(t: TierName): LlmTierConfig | null {
  return settingsStore.snapshot ? settingsStore.snapshot[t] : null;
}

/** 三档均未配置任何 Key → 必选约束引导。 */
const noneConfigured = (): boolean => {
  const s = settingsStore.snapshot;
  return !!s && !s.main.hasKey && !s.memory.hasKey && !s.fallback.hasKey;
};

/** 当前档（fallback）未配置 → 回落提示。 */
const showFallbackHint = (): boolean =>
  activeTier.value === "fallback" && !forms.fallback.keyHas && !forms.fallback.baseUrl;

function providerGroup(required: boolean): ProviderPreset[] {
  return providers.value.filter((p) => p.required === required);
}

function selectProvider(id: string): void {
  const preset = providers.value.find((p) => p.id === id);
  if (!preset) return;
  selected[activeTier.value] = id;
  const f = forms[activeTier.value];
  f.baseUrl = preset.baseUrl;
  f.model = preset.defaultModel;
}

function isQuotaCandidate(p: ProviderPreset): boolean {
  return QUOTA_CANDIDATE_IDS.has(p.id) || p.costTier === "free";
}

async function save(): Promise<void> {
  const tier = activeTier.value;
  const f = forms[tier];
  const payload: SaveTierPayload = {
    baseUrl: f.baseUrl,
    model: f.model,
    modelCustom: f.modelCustom,
    apiKey: f.keyInput || undefined,
  };
  saving.value = true;
  try {
    await saveTyped(tier, payload);
    const snap = await getTyped();
    applySnapshot(snap);
    // 刷新指纹 + 清空输入框（已保存，回显指纹）
    const cfg = tierConfig(tier);
    if (cfg) {
      f.keyFingerprint = cfg.keyFingerprint;
      f.keyHas = cfg.hasKey;
    }
    f.keyInput = "";
    pushToast("已保存");
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "保存失败", "error");
  } finally {
    saving.value = false;
  }
}

async function toggleQuota(useBuiltin: boolean): Promise<void> {
  try {
    await setQuotaMode(useBuiltin ? 1 : 2);
    const snap = await getTyped();
    applySnapshot(snap);
    pushToast(useBuiltin ? "已切换到内置额度" : "已切换到自带 Key");
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "切换失败", "error");
  }
}

onMounted(async (): Promise<void> => {
  try {
    providers.value = await listProviders();
  } catch (e) {
    console.warn("[llm] 服务商列表拉取失败", e);
  }
  if (initialized.value) return;
  try {
    const snap = await getTyped();
    applySnapshot(snap);
    (["main", "memory", "fallback"] as TierName[]).forEach((t): void => {
      const cfg = snap[t];
      const f = forms[t];
      f.baseUrl = cfg.baseUrl;
      f.model = cfg.model;
      f.modelCustom = cfg.modelCustom;
      f.keyFingerprint = cfg.keyFingerprint;
      f.keyHas = cfg.hasKey;
      const hit = providers.value.find(
        (p) => p.baseUrl === cfg.baseUrl && p.defaultModel === cfg.model,
      );
      selected[t] = hit?.id ?? null;
    });
    initialized.value = true;
  } catch (e) {
    console.warn("[llm] 配置拉取失败", e);
  }
});
</script>

<template>
  <div class="llm-panel">
    <!-- 卡片 1：服务商与凭据 -->
    <AppCard title="服务商与凭据">
      <HintBox
        v-if="noneConfigured()"
        tone="warn"
      >
        至少配置 DeepSeek 或通义千问之一，即可独立支撑「主对话 + 记忆沉淀」全流程。
      </HintBox>

      <SegmentedControl
        v-model="activeTier"
        :options="TIERS"
      />

      <div class="llm-panel__providers">
        <div class="llm-panel__group">
          <span class="llm-panel__group-label">★ 必选</span>
          <ProviderChip
            v-for="p in providerGroup(true)"
            :key="p.id"
            :label="p.label"
            :required="true"
            :cost-tier="p.costTier"
            :selected="selected[activeTier] === p.id"
            :quota-candidate="isQuotaCandidate(p)"
            @select="selectProvider(p.id)"
          />
        </div>
        <details class="llm-panel__optional">
          <summary>＋ 添加低成本服务商</summary>
          <div class="llm-panel__group">
            <ProviderChip
              v-for="p in providerGroup(false)"
              :key="p.id"
              :label="p.label"
              :required="false"
              :cost-tier="p.costTier"
              :selected="selected[activeTier] === p.id"
              :quota-candidate="isQuotaCandidate(p)"
              @select="selectProvider(p.id)"
            />
          </div>
        </details>
      </div>

      <div class="llm-panel__row">
        <AppInput
          v-model="forms[activeTier].baseUrl"
          label="Base URL"
          placeholder="https://api.example.com/v1"
        />
        <AppInput
          v-model="forms[activeTier].model"
          label="模型名称"
          placeholder="deepseek-chat"
        />
      </div>

      <div class="llm-panel__key">
        <AppInput
          v-model="forms[activeTier].keyInput"
          type="password"
          label="API Key"
          :placeholder="
            forms[activeTier].keyHas
              ? `已配置 ····${forms[activeTier].keyFingerprint ?? ''}`
              : '粘贴服务商 Key（留空=不改动）'
          "
        />
        <StatusBadge
          v-if="forms[activeTier].keyHas"
          tone="ok"
          text="🔒 已由系统凭据管理器加密"
        />
      </div>

      <HintBox
        v-if="showFallbackHint()"
        tone="info"
      >
        兜底档未配置时将自动回落到必选模型，主对话仍可正常进行。
      </HintBox>

      <div class="llm-panel__actions">
        <AppButton
          variant="primary"
          :loading="saving"
          @click="save"
        >
          保存
        </AppButton>
      </div>

      <HintBox tone="info">
        计费说明：各服务商按调用量计费，价格以官方控制台为准（DeepSeek / 通义千问 / GLM /
        SiliconFlow 等）。内置体验额度由应用统一代理，切换「自带
        Key」后费用由你的账户承担。
      </HintBox>
    </AppCard>

    <!-- 卡片 2：内置体验额度 -->
    <AppCard title="内置体验额度">
      <div class="llm-panel__quota">
        <div class="llm-panel__quota-head">
          <span>剩余体验额度</span>
          <StatusBadge
            :tone="settingsStore.snapshot?.isCustomQuotaUsed === 2 ? 'warn' : 'ok'"
            :text="
              settingsStore.snapshot?.isCustomQuotaUsed === 2
                ? '使用自带 Key'
                : '内置额度启用'
            "
          />
        </div>
        <QuotaBar :ratio="1" />
        <p class="llm-panel__quota-note">
          体验额度用量随后端开放（当前为启用状态占位）。关闭即完全切到自带 Key。
        </p>
      </div>
      <AppToggle
        :model-value="settingsStore.snapshot?.isCustomQuotaUsed !== 2"
        label="使用内置额度"
        @update:model-value="toggleQuota($event)"
      />
    </AppCard>
  </div>
</template>

<style scoped>
.llm-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.llm-panel__providers {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.llm-panel__group {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.llm-panel__group-label {
  font-size: 12px;
  font-weight: 700;
  color: var(--ink-soft);
}
.llm-panel__optional summary {
  cursor: pointer;
  font-size: 13px;
  color: var(--matcha-deep);
  font-weight: 600;
}
.llm-panel__row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.llm-panel__key {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.llm-panel__actions {
  display: flex;
  justify-content: flex-end;
}
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
</style>

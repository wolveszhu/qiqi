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
import {
  deleteCustomModel,
  getTyped,
  listCustomModels,
  listProviders,
  moveCustomModel,
  saveCustomModel,
  saveTyped,
  setQuotaMode,
} from "../../../api/settings";
import { pushToast } from "../../../composables/useToast";
import { applySnapshot, settingsStore } from "../../../state/settingsStore";
import type {
  CustomModelDto,
  LlmTierConfig,
  ProviderPreset,
  SaveCustomModelPayload,
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

// ---- 自定义多模型管理（优先于内置；编排顺序 = 列表顺序）----
const customs = ref<CustomModelDto[]>([]);
const customEditorOpen = ref(false);
const customSaving = ref(false);
const customForm = reactive({
  id: null as number | null,
  name: "",
  baseUrl: "",
  modelsText: "",
  keyInput: "",
  /** 编辑中是否已存 Key（决定输入框 placeholder 语义）。 */
  keyHas: false,
  keyFingerprint: null as string | null,
  enabled: true,
});

/** 刷新自定义平台列表（失败 toast——不让用户以为保存成功）。 */
async function refreshCustoms(): Promise<void> {
  try {
    customs.value = await listCustomModels();
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "自定义模型列表加载失败", "error");
  }
}

/** 打开编辑器（带参=编辑，无参=新增）。 */
function openCustomEditor(item?: CustomModelDto): void {
  customEditorOpen.value = true;
  customForm.id = item?.id ?? null;
  customForm.name = item?.name ?? "";
  customForm.baseUrl = item?.baseUrl ?? "";
  customForm.modelsText = (item?.models ?? []).join("\n");
  customForm.keyInput = "";
  customForm.keyHas = item?.hasKey ?? false;
  customForm.keyFingerprint = item?.keyFingerprint ?? null;
  customForm.enabled = item?.enabled ?? true;
}

/** 解析模型列表输入（换行/逗号分隔，去空白去空项）。 */
function parseModelsInput(text: string): string[] {
  return text
    .split(/[\n,]/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

async function submitCustom(): Promise<void> {
  const payload: SaveCustomModelPayload = {
    id: customForm.id ?? undefined,
    name: customForm.name.trim(),
    baseUrl: customForm.baseUrl.trim(),
    models: parseModelsInput(customForm.modelsText),
    // 留空 = 不改动存量 Key（新增时即不配 Key，由 UI 提示补配）
    apiKey: customForm.keyInput || undefined,
    enabled: customForm.enabled,
  };
  customSaving.value = true;
  try {
    await saveCustomModel(payload);
    await refreshCustoms();
    customEditorOpen.value = false;
    pushToast("已保存");
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "保存失败", "error");
  } finally {
    customSaving.value = false;
  }
}

/** 删除平台（二次确认由列表按钮 aria 文案承担；误删可重新添加）。 */
async function removeCustom(id: number): Promise<void> {
  try {
    await deleteCustomModel(id);
    await refreshCustoms();
    pushToast("已删除");
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "删除失败", "error");
  }
}

/** 上移/下移（调整编排优先级；越界时 Rust 静默成功）。 */
async function moveCustom(id: number, up: boolean): Promise<void> {
  try {
    await moveCustomModel(id, up);
    await refreshCustoms();
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "调整顺序失败", "error");
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
    pushToast(e instanceof Error ? e.message : "切换启用态失败", "error");
  }
}

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
  await refreshCustoms();
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

    <!-- 卡片 2：自定义多模型（优先于内置；顺序 failover） -->
    <AppCard title="自定义大模型（优先使用）">
      <HintBox tone="info">
        按列表顺序依次调用，某个失败自动切下一个；全部失败时回落到内置免费模型
        （需保持下方「使用内置额度」开启）。
      </HintBox>

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
              c.models.length ? c.models.join(" · ") : "未填写模型"
            }}</span>
            <span class="custom-item__url">{{ c.baseUrl }}</span>
          </div>
          <div class="custom-item__side">
            <StatusBadge
              :tone="c.hasKey ? 'ok' : 'warn'"
              :text="c.hasKey ? `🔒 ····${c.keyFingerprint ?? ''}` : '未配置 Key'"
            />
            <AppToggle
              :model-value="c.enabled"
              @update:model-value="(v: boolean) => toggleCustomEnabled(c, v)"
            />
            <button
              class="custom-item__btn"
              type="button"
              aria-label="上移"
              :disabled="i === 0"
              @click="moveCustom(c.id, true)"
            >
              ↑
            </button>
            <button
              class="custom-item__btn"
              type="button"
              aria-label="下移"
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
              编辑
            </button>
            <button
              class="custom-item__btn custom-item__btn--danger"
              type="button"
              aria-label="删除平台"
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
        还没有自定义平台。添加后优先于内置模型使用。
      </p>

      <div class="custom-actions">
        <AppButton
          variant="ghost"
          @click="openCustomEditor()"
        >
          ＋ 新增平台
        </AppButton>
      </div>

      <div
        v-if="customEditorOpen"
        class="custom-editor"
      >
        <AppInput
          v-model="customForm.name"
          label="平台名称"
          placeholder="我的 DeepSeek"
          :maxlength="20"
        />
        <AppInput
          v-model="customForm.baseUrl"
          label="Base URL"
          placeholder="https://api.example.com/v1"
        />
        <div class="custom-editor__models">
          <span class="custom-editor__label">模型列表（每行一个，或逗号分隔）</span>
          <textarea
            v-model="customForm.modelsText"
            class="custom-editor__textarea"
            rows="3"
            placeholder="deepseek-chat&#10;deepseek-reasoner"
          ></textarea>
        </div>
        <AppInput
          v-model="customForm.keyInput"
          type="password"
          label="API Key"
          :placeholder="
            customForm.keyHas
              ? `已配置 ····${customForm.keyFingerprint ?? ''}（留空=不改动）`
              : '粘贴服务商 Key（留空=暂不配置）'
          "
        />
        <AppToggle
          :model-value="customForm.enabled"
          label="启用该平台"
          @update:model-value="(v: boolean) => (customForm.enabled = v)"
        />
        <div class="custom-editor__actions">
          <AppButton
            variant="primary"
            :loading="customSaving"
            @click="submitCustom"
          >
            保存
          </AppButton>
          <AppButton
            variant="ghost"
            @click="customEditorOpen = false"
          >
            取消
          </AppButton>
        </div>
      </div>
    </AppCard>

    <!-- 卡片 3：内置体验额度 -->
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

/* ---- 自定义多模型管理 ---- */
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
  transition: background var(--dur-fast) var(--ease-out);
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
.custom-actions {
  display: flex;
  justify-content: flex-end;
}
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
.custom-editor__textarea {
  resize: vertical;
  border-radius: 12px;
  border: 1px solid var(--border-soft);
  background: var(--surface-panel);
  color: var(--text-primary);
  padding: 8px 10px;
  font: 500 13px/1.5 var(--font-ui);
  outline: none;
}
.custom-editor__textarea:focus {
  border-color: var(--accent-rose);
}
.custom-editor__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>

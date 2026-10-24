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
import { onMounted, reactive, ref } from "vue";

import AppButton from "../../../components/ui/AppButton.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import AppInput from "../../../components/ui/AppInput.vue";
import AppToggle from "../../../components/ui/AppToggle.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import StatusBadge from "../../../components/ui/StatusBadge.vue";
import ModelChips from "../../../components/settings/ModelChips.vue";
import ProviderChip from "../../../components/settings/ProviderChip.vue";
import QuotaBar from "../../../components/settings/QuotaBar.vue";
import {
  deleteCustomModel,
  getTyped,
  listCustomModels,
  listProviders,
  moveCustomModel,
  saveCustomModel,
  setQuotaMode,
} from "../../../api/settings";
import { pushToast } from "../../../composables/useToast";
import { applySnapshot, settingsStore } from "../../../state/settingsStore";
import type {
  CustomModelDto,
  ProviderPreset,
  SaveCustomModelPayload,
} from "../../../api/types";

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

/** 刷新自定义平台列表（失败 toast——不让用户以为保存成功）。 */
async function refreshCustoms(): Promise<void> {
  try {
    customs.value = await listCustomModels();
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "自定义模型列表加载失败", "error");
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

/** 点击平台预设 → 预填编辑器（名称/URL/常见模型；Key 永远手输）。 */
function openPresetEditor(preset: ProviderPreset): void {
  openCustomEditor();
  customForm.presetId = preset.id;
  customForm.name = preset.label;
  customForm.baseUrl = preset.baseUrl;
  customForm.models = [...preset.defaultModels];
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
    pushToast("已保存");
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "保存失败", "error");
  } finally {
    customSaving.value = false;
  }
}

/** 删除平台。 */
async function removeCustom(id: number): Promise<void> {
  try {
    await deleteCustomModel(id);
    await refreshCustoms();
    pushToast("已删除");
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "删除失败", "error");
  }
}

/** 上移/下移一位（调整编排优先级；越界时 Rust 静默成功）。 */
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
});
</script>

<template>
  <div class="llm-panel">
    <!-- 卡片 1：自定义大模型服务商（平台预设 + 平台列表 + 编辑器） -->
    <AppCard title="自定义大模型服务商">
      <HintBox tone="info">
        按列表顺序依次调用，某个失败自动切下一个；全部失败时回落到内置免费模型
        （需保持下方「使用内置额度」开启）。记忆沉淀档由后台自动选择，无需配置。
      </HintBox>

      <!-- 平台预设：点击即预填（URL 可改、模型可增删、Key 手输） -->
      <div class="preset-block">
        <span class="preset-block__label">选择平台快速开始</span>
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
        还没有配置平台。点击上方平台快速开始，或手动新增。
      </p>

      <div class="custom-actions">
        <AppButton
          variant="ghost"
          @click="openCustomEditor()"
        >
          ＋ 手动新增平台
        </AppButton>
      </div>

      <!-- 编辑器 -->
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
          <span class="custom-editor__label">大模型（回车 / 逗号添加，点 ✕ 删除）</span>
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
            至少添加一个模型；点击预设平台会自动带上常见模型。
          </p>
        </div>
        <AppInput
          v-model="customForm.keyInput"
          type="password"
          label="API Key（手动输入）"
          :placeholder="
            customForm.keyHas
              ? `已配置 ····${customForm.keyFingerprint ?? ''}（留空=不改动）`
              : '粘贴该平台的 API Key'
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

      <HintBox tone="info">
        计费说明：各服务商按调用量计费，价格以官方控制台为准。关闭「使用内置额度」后
        全部调用走你自己配置的平台与 Key。
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
          体验额度用量随后端开放（当前为启用状态占位）。关闭即完全切到自有平台。
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
.custom-actions {
  display: flex;
  justify-content: flex-end;
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
</style>

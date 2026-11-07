<script setup lang="ts">
/**
 * 服务商 chip（设计方案 §5.2 / §4.2）：选中态深底白字 + 蜜桃橙圆点；
 * ★ 必选前缀 + 成本档徽标（低/极低/免费）+ 可选"内置额度池候选"角标。
 * 点击回填 Base URL/模型到所在档位表单（事件上抛，业务无耦合）。
 */
import { useI18n } from "vue-i18n";

const { t } = useI18n();

/**
 * Rust `PROVIDER_PRESETS.cost_tier` 下发的原文 → i18n 键。
 * 后端下发的就是中文文案（如 "免费/极低"），故按原文精确映射；
 * 未知档位原样回退，防止后续新增档位丢展示。
 */
const COST_LABEL_KEYS: Record<string, string> = {
  低: "provider.costLow",
  极低: "provider.costLower",
  免费: "provider.costFree",
  较高: "provider.costHigh",
  "低-中": "provider.costLowMid",
  "免费/极低": "provider.costFreeLower",
  "免费/低": "provider.costFreeLow",
};

withDefaults(
  defineProps<{
    label: string;
    required?: boolean;
    costTier: string;
    selected: boolean;
    /** 内置额度池候选（呼应 C3 EMBEDDED_QUOTA_BASE_URL）。 */
    quotaCandidate?: boolean;
  }>(),
  { required: false, quotaCandidate: false },
);

const emit = defineEmits<{ (e: "select"): void }>();

function costLabel(tier: string): string {
  const key = COST_LABEL_KEYS[tier];
  return key ? t(key) : tier;
}
</script>

<template>
  <button
    class="p-chip"
    :class="{ 'is-selected': selected }"
    :aria-pressed="selected"
    type="button"
    @click="emit('select')"
  >
    <span
      v-if="required"
      class="p-chip__star"
      aria-hidden="true"
      >★</span
    >
    <span
      class="p-chip__dot"
      aria-hidden="true"
    ></span>
    <span class="p-chip__label">{{ label }}</span>
    <span class="p-chip__cost">{{ costLabel(costTier) }}</span>
    <span
      v-if="quotaCandidate"
      class="p-chip__quota"
      >{{ t("provider.quotaPool") }}</span
    >
  </button>
</template>

<style scoped>
.p-chip {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 9px 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--bg-window);
  cursor: pointer;
  font-size: 12.5px;
  font-weight: 700;
  color: var(--ink);
  transition:
    background var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    color 160ms ease;
}
.p-chip:hover {
  border-color: var(--matcha);
}
.p-chip.is-selected {
  background: var(--ink);
  border-color: var(--ink);
  color: var(--bg-window);
}
.p-chip__star {
  color: var(--butter);
  font-size: 11px;
}
.p-chip__dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--peach);
  flex: none;
}
.p-chip__cost {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--bg-deep);
  color: var(--ink-soft);
}
.p-chip.is-selected .p-chip__cost {
  background: rgba(255, 255, 255, 0.16);
  color: var(--bg-window);
}
.p-chip__quota {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--matcha-soft);
  color: var(--matcha-deep);
}
</style>

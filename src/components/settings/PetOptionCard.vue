<script setup lang="ts">
/**
 * 形象选项卡（设计方案 §5.2 / §4.1）：选中态 2px matcha 边 + 浅绿底 + 「✓ 使用中」角标；
 * disabled 供尚未实现的形象灰显（当前两套内置形象均已实现，故不传）；
 * dashed 变体为「导入自定义形象」占位（＋）。点击上抛 select 事件，业务无耦合。
 */
import { useI18n } from "vue-i18n";

const { t } = useI18n();

withDefaults(
  defineProps<{
    name: string;
    sub?: string;
    emoji?: string;
    selected?: boolean;
    disabled?: boolean;
    dashed?: boolean;
  }>(),
  { selected: false, disabled: false, dashed: false },
);

const emit = defineEmits<{ (e: "select"): void }>();
</script>

<template>
  <button
    class="pet-opt"
    :class="{ 'is-selected': selected, 'is-disabled': disabled, 'is-dashed': dashed }"
    :disabled="disabled"
    type="button"
    @click="emit('select')"
  >
    <span class="pet-opt__avatar">{{ emoji ?? "🐱" }}</span>
    <span class="pet-opt__info">
      <span class="pet-opt__name">{{ name }}</span>
      <span
        v-if="sub"
        class="pet-opt__sub"
        >{{ sub }}</span
      >
    </span>
    <span
      v-if="selected"
      class="pet-opt__check"
      >{{ t("petOption.inUse") }}</span
    >
  </button>
</template>

<style scoped>
.pet-opt {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 2px solid var(--line);
  border-radius: 16px;
  background: var(--bg-window);
  cursor: pointer;
  text-align: left;
  transition:
    border-color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}
.pet-opt:hover:not(.is-disabled) {
  border-color: var(--matcha);
}
.pet-opt.is-selected {
  border-color: var(--matcha);
  background: var(--matcha-soft);
}
.pet-opt.is-disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.pet-opt.is-dashed {
  border-style: dashed;
  border-color: var(--line);
  color: var(--matcha-deep);
}
.pet-opt__avatar {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 24px;
  background: var(--bg-soft);
  flex: none;
}
.pet-opt__info {
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.pet-opt__name {
  font-size: 14px;
  font-weight: 700;
  color: var(--ink);
}
.pet-opt__sub {
  font-size: 11.5px;
  color: var(--ink-soft);
}
.pet-opt__check {
  margin-left: auto;
  font-size: 11px;
  font-weight: 700;
  color: var(--matcha-deep);
}
</style>

<script setup lang="ts">
/**
 * 形象选项卡（设计方案 §5.2 / §4.1）：选中态 2px matcha 边 + 浅绿底 + 「✓ 使用中」角标；
 * disabled 供尚未实现的形象灰显（当前两套内置形象均已实现，故不传）；
 * dashed 变体为「导入自定义形象」占位（＋）。点击上抛 select 事件，业务无耦合。
 * 自定义形象：传 `image`（dataUrl）显示真实缩略图，传 `deletable` 时右上角出现删除按钮，
 * 点击删除上抛 delete 事件（阻止冒泡，不会误触 select）。
 */
import { useI18n } from "vue-i18n";

const { t } = useI18n();

withDefaults(
  defineProps<{
    name: string;
    sub?: string;
    emoji?: string;
    image?: string;
    selected?: boolean;
    disabled?: boolean;
    dashed?: boolean;
    deletable?: boolean;
  }>(),
  { selected: false, disabled: false, dashed: false, deletable: false },
);

const emit = defineEmits<{ (e: "select"): void; (e: "delete"): void }>();
</script>

<template>
  <button
    class="pet-opt"
    :class="{ 'is-selected': selected, 'is-disabled': disabled, 'is-dashed': dashed }"
    :disabled="disabled"
    type="button"
    @click="emit('select')"
  >
    <span class="pet-opt__avatar">
      <img
        v-if="image"
        class="pet-opt__img"
        :src="image"
        :alt="name"
      />
      <template v-else>{{ emoji ?? "🐱" }}</template>
    </span>
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
    <button
      v-if="deletable"
      class="pet-opt__del"
      type="button"
      :aria-label="t('avatar.deleteAvatar')"
      @click.stop="emit('delete')"
    >
      ×
    </button>
  </button>
</template>

<style scoped>
.pet-opt {
  position: relative;
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
  overflow: hidden;
}
.pet-opt__img {
  width: 100%;
  height: 100%;
  object-fit: contain;
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
.pet-opt__del {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 18px;
  height: 18px;
  display: grid;
  place-items: center;
  padding: 0;
  border: none;
  border-radius: 999px;
  background: var(--peach-deep);
  color: #fff;
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
  opacity: 0.85;
  transition: opacity var(--dur-fast) var(--ease-out);
}
.pet-opt__del:hover {
  opacity: 1;
}
</style>

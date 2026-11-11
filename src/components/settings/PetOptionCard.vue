<script setup lang="ts">
/**
 * 形象选项卡（设计方案 §5.2 / §4.1）：选中态 2px matcha 边 + 浅绿底 + 「✓ 使用中」角标；
 * disabled 供尚未实现的形象灰显（当前两套内置形象均已实现，故不传）；
 * dashed 变体为「导入自定义形象」占位（＋）。点击上抛 select 事件，业务无耦合。
 * 自定义形象：传 `image`（dataUrl）显示真实缩略图，传 `deletable` 时展示明显的「删除」按钮；
 * 为防误触，点「删除」后进入二次确认态（确认删除 / 取消），确认才上抛 delete 事件。
 */
import { ref } from "vue";
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

/** 二次确认态：点「删除」→ true，展示「确认删除？[删除][取消]」，避免误触。 */
const confirming = ref(false);

function onConfirmDelete(): void {
  confirming.value = false;
  emit("delete");
}
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
    <span
      v-if="deletable"
      class="pet-opt__del-wrap"
    >
      <button
        v-if="!confirming"
        class="pet-opt__del"
        type="button"
        :aria-label="t('avatar.deleteAvatar')"
        @click.stop="confirming = true"
      >
        {{ t("avatar.deleteLabel") }}
      </button>
      <span
        v-else
        class="pet-opt__confirm"
      >
        <span class="pet-opt__confirm-text">{{ t("avatar.deleteConfirmShort") }}</span>
        <button
          class="pet-opt__confirm-yes"
          type="button"
          @click.stop="onConfirmDelete"
        >
          {{ t("avatar.deleteConfirmYes") }}
        </button>
        <button
          class="pet-opt__confirm-no"
          type="button"
          @click.stop="confirming = false"
        >
          {{ t("common.cancel") }}
        </button>
      </span>
    </span>
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
.pet-opt__del-wrap {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: flex;
  align-items: center;
}
.pet-opt__del {
  padding: 4px 12px;
  border: 1px solid var(--peach-deep);
  border-radius: 999px;
  background: var(--bg-window);
  color: var(--peach-deep);
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
}
.pet-opt__del:hover {
  background: var(--peach-deep);
  color: #fff;
}
.pet-opt__confirm {
  display: flex;
  align-items: center;
  gap: 6px;
}
.pet-opt__confirm-text {
  font-size: 12px;
  color: var(--ink-soft);
  white-space: nowrap;
}
.pet-opt__confirm-yes {
  padding: 3px 10px;
  border: none;
  border-radius: 999px;
  background: var(--peach-deep);
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}
.pet-opt__confirm-yes:hover {
  filter: brightness(0.95);
}
.pet-opt__confirm-no {
  padding: 3px 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--bg-window);
  color: var(--ink-soft);
  font-size: 12px;
  cursor: pointer;
}
</style>

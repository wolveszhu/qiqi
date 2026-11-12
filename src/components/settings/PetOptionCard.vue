<script setup lang="ts">
/**
 * 形象选项卡（设计方案 §5.2 / §4.1）：选中态 2px matcha 边 + 浅绿底 + 「✓ 使用中」角标；
 * disabled 供尚未实现的形象灰显（当前两套内置形象均已实现，故不传）；
 * dashed 变体为「导入自定义形象」占位（＋）。点击上抛 select 事件，业务无耦合。
 * 自定义形象：传 `image`（dataUrl）显示真实缩略图，传 `deletable` 时左上角出现删除图标；
 * 点图标弹出确认弹窗（取消 / 删除），避免误触，确认才上抛 delete 事件。
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
      class="pet-opt__del"
      role="button"
      tabindex="0"
      :aria-label="t('avatar.deleteAvatar')"
      @click.stop="confirming = true"
      @keydown.enter.stop.prevent="confirming = true"
      @keydown.space.stop.prevent="confirming = true"
    >
      <svg
        class="pet-opt__del-icon"
        viewBox="0 0 24 24"
        aria-hidden="true"
      >
        <path
          d="M3 6h18M8 6V4h8v2m-9 0v14a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V6M10 11v6M14 11v6"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </span>
    <Teleport to="body">
      <div
        v-if="confirming"
        class="pet-del-modal"
        @click.self="confirming = false"
      >
        <div
          class="pet-del-modal__box"
          role="dialog"
          aria-modal="true"
        >
          <p class="pet-del-modal__text">{{ t("avatar.deleteConfirm") }}</p>
          <div class="pet-del-modal__actions">
            <button
              class="pet-del-modal__cancel"
              type="button"
              @click="confirming = false"
            >
              {{ t("common.cancel") }}
            </button>
            <button
              class="pet-del-modal__confirm"
              type="button"
              @click="onConfirmDelete"
            >
              {{ t("avatar.deleteConfirmYes") }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
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
  top: 6px;
  left: 6px;
  z-index: 1;
  width: 24px;
  height: 24px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--bg-window);
  color: var(--peach-deep);
  cursor: pointer;
  opacity: 0.92;
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out),
    opacity var(--dur-fast) var(--ease-out);
}
.pet-opt__del:hover {
  background: var(--peach-deep);
  color: #fff;
  opacity: 1;
}
.pet-opt__del:focus-visible {
  outline: 2px solid var(--matcha);
  outline-offset: 1px;
}
.pet-opt__del-icon {
  width: 14px;
  height: 14px;
}
/* 删除确认弹窗（Teleport 到 body，固定全屏遮罩 + 居中卡片） */
.pet-del-modal {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  background: rgba(0, 0, 0, 0.4);
}
.pet-del-modal__box {
  width: min(320px, 100%);
  padding: 20px;
  border-radius: 16px;
  background: var(--bg-window);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
}
.pet-del-modal__text {
  margin: 0 0 18px;
  font-size: 14px;
  line-height: 1.6;
  color: var(--ink);
}
.pet-del-modal__actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
.pet-del-modal__cancel {
  padding: 8px 16px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--bg-window);
  color: var(--ink-soft);
  font-size: 13px;
  cursor: pointer;
}
.pet-del-modal__cancel:hover {
  background: var(--bg-soft);
}
.pet-del-modal__confirm {
  padding: 8px 16px;
  border: none;
  border-radius: 10px;
  background: var(--peach-deep);
  color: #fff;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
}
.pet-del-modal__confirm:hover {
  filter: brightness(0.95);
}
</style>

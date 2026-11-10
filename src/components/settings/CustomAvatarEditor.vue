<script setup lang="ts">
/**
 * 自定义形象微调面板（仅当选中自定义卡时出现）。
 *
 * 提供：缩放滑杆 + 锚点 X/Y 滑杆（实时预览 + 防抖落库）+ 重命名 + 删除。
 * 滑杆拖动即时改注册表内存并重建预览（`previewKey` 强制 PreviewStage 重建），
 * 落库走 250ms 防抖命令，避免高频 invoke。
 *
 * 边界：仅调 Rust 命令与注册表；不接触文件系统（图片字节已在注册表内存中）。
 */
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import PreviewStage from "./PreviewStage.vue";
import {
  deleteCustomAvatar,
  renameCustomAvatar,
  updateCustomAvatarConfig,
} from "../../api/customAvatar";
import {
  getCustomConfig,
  removeCustomConfig,
  updateCustomConfigInMemory,
} from "../../renderer/customAvatarRegistry";
import { pushToast } from "../../composables/useToast";

const props = defineProps<{ avatarId: string }>();
const emit = defineEmits<{
  (e: "updated"): void;
  (e: "deleted"): void;
}>();

const { t } = useI18n();

const uuid = computed(() =>
  props.avatarId.startsWith("custom:")
    ? props.avatarId.slice("custom:".length)
    : props.avatarId,
);

const scale = ref(1);
const anchorX = ref(0.5);
const anchorY = ref(0.5);
const name = ref("");

/** 切换被编辑形象时从注册表回填本地值。 */
function syncLocalFromRegistry(): void {
  const c = getCustomConfig(uuid.value);
  if (!c) return;
  scale.value = c.scale;
  anchorX.value = c.anchorX;
  anchorY.value = c.anchorY;
  name.value = c.name;
}
watch(() => props.avatarId, syncLocalFromRegistry, { immediate: true });

/** 重建预览的代号（key 一变即销毁旧实例、挂载新渲染器读最新配置）。 */
const previewKey = ref(0);
let saveTimer: ReturnType<typeof setTimeout> | null = null;

/** 几何滑杆：即时改内存（预览重建）+ 节流落库。 */
function onGeometry(): void {
  updateCustomConfigInMemory(uuid.value, {
    scale: scale.value,
    anchorX: anchorX.value,
    anchorY: anchorY.value,
  });
  previewKey.value++;
  emit("updated");
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    void updateCustomAvatarConfig(
      uuid.value,
      scale.value,
      anchorX.value,
      anchorY.value,
    ).catch((e: unknown) =>
      pushToast(e instanceof Error ? e.message : t("common.saveFailed"), "error"),
    );
  }, 250);
}

let nameTimer: ReturnType<typeof setTimeout> | null = null;
function onRename(): void {
  const c = getCustomConfig(uuid.value);
  const v = name.value.trim();
  if (!v) {
    name.value = c?.name ?? "";
    return;
  }
  updateCustomConfigInMemory(uuid.value, { name: v });
  emit("updated");
  if (nameTimer) clearTimeout(nameTimer);
  nameTimer = setTimeout(() => {
    void renameCustomAvatar(uuid.value, v).catch((e: unknown) =>
      pushToast(e instanceof Error ? e.message : t("common.saveFailed"), "error"),
    );
  }, 250);
}

async function onDelete(): Promise<void> {
  if (!window.confirm(t("avatar.deleteConfirm"))) return;
  try {
    await deleteCustomAvatar(uuid.value);
    removeCustomConfig(uuid.value); // Rust 已回退激活态并发 pet:renderer:changed
    pushToast(t("avatar.deleteOk"));
    emit("deleted");
  } catch (e: unknown) {
    pushToast(e instanceof Error ? e.message : t("common.saveFailed"), "error");
  }
}
</script>

<template>
  <div class="custom-editor">
    <div class="custom-editor__preview">
      <PreviewStage
        :key="previewKey"
        :renderer-id="avatarId"
      />
    </div>
    <div class="custom-editor__controls">
      <label class="ctl">
        <span class="ctl__title">{{ t("avatar.scaleLabel") }}</span>
        <input
          class="ctl__slider"
          type="range"
          min="0.2"
          max="3"
          step="0.05"
          :value="scale"
          @input="
            scale = Number(($event.target as HTMLInputElement).value);
            onGeometry();
          "
        />
        <span class="ctl__val">{{ Math.round(scale * 100) }}%</span>
      </label>
      <label class="ctl">
        <span class="ctl__title">{{ t("avatar.anchorXLabel") }}</span>
        <input
          class="ctl__slider"
          type="range"
          min="0"
          max="1"
          step="0.01"
          :value="anchorX"
          @input="
            anchorX = Number(($event.target as HTMLInputElement).value);
            onGeometry();
          "
        />
        <span class="ctl__val">{{ Math.round(anchorX * 100) }}%</span>
      </label>
      <label class="ctl">
        <span class="ctl__title">{{ t("avatar.anchorYLabel") }}</span>
        <input
          class="ctl__slider"
          type="range"
          min="0"
          max="1"
          step="0.01"
          :value="anchorY"
          @input="
            anchorY = Number(($event.target as HTMLInputElement).value);
            onGeometry();
          "
        />
        <span class="ctl__val">{{ Math.round(anchorY * 100) }}%</span>
      </label>
      <p class="custom-editor__hint">{{ t("avatar.anchorHint") }}</p>
      <label class="ctl ctl--name">
        <span class="ctl__title">{{ t("avatar.renameLabel") }}</span>
        <input
          class="ctl__input"
          type="text"
          :value="name"
          :maxlength="20"
          @input="
            name = ($event.target as HTMLInputElement).value;
            onRename();
          "
        />
      </label>
      <button
        class="custom-editor__del"
        type="button"
        @click="onDelete"
      >
        {{ t("avatar.deleteAvatar") }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.custom-editor {
  display: grid;
  grid-template-columns: 200px 1fr;
  gap: 16px;
  align-items: start;
}
.custom-editor__preview {
  display: flex;
  justify-content: center;
}
.custom-editor__controls {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.ctl {
  display: grid;
  grid-template-columns: 96px 1fr 52px;
  align-items: center;
  gap: 10px;
}
.ctl__title {
  font-size: 13px;
  color: var(--ink);
  font-weight: 600;
}
.ctl__slider {
  width: 100%;
  accent-color: var(--matcha-deep);
}
.ctl__val {
  font-size: 12px;
  color: var(--ink-soft);
  text-align: right;
  font-variant-numeric: tabular-nums;
}
.ctl--name {
  grid-template-columns: 96px 1fr;
}
.ctl__input {
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 7px 10px;
  font-size: 13px;
  color: var(--ink);
  background: var(--bg-window);
  outline: none;
  transition: border-color var(--dur-fast) var(--ease-out);
}
.ctl__input:focus {
  border-color: var(--matcha);
}
.custom-editor__hint {
  font-size: 12px;
  color: var(--ink-soft);
  line-height: 1.5;
  margin: 0;
}
.custom-editor__del {
  align-self: flex-start;
  margin-top: 4px;
  padding: 7px 14px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: var(--bg-window);
  color: var(--peach-deep);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition:
    background var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}
.custom-editor__del:hover {
  background: var(--peach-soft);
}
.custom-editor__del:active {
  transform: scale(0.97);
}
@media (max-width: 520px) {
  .custom-editor {
    grid-template-columns: 1fr;
  }
}
</style>

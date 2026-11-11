<script setup lang="ts">
/**
 * 桌宠面板（设计方案 §4.1）：名字即改即存（pet_update_state, debounce）+ 形象选择 + 预览。
 *
 * 形象清单 = 内置（factory `listRenderers` 派生）+ 自定义（customAvatarRegistry 合并）；
 * 导入走 `tauri-plugin-dialog` 取路径 → Rust 校验/拷贝/落库 → 注册表即时刷新。
 * 自定义形象卡的缩略图直接由注册表 dataUrl 渲染，删除按钮在网格内就地触发（不再展开微调面板）。
 */
import { open } from "@tauri-apps/plugin-dialog";
import { emit } from "@tauri-apps/api/event";
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import AppCard from "../../../components/ui/AppCard.vue";
import AppInput from "../../../components/ui/AppInput.vue";
import AppToggle from "../../../components/ui/AppToggle.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import PetOptionCard from "../../../components/settings/PetOptionCard.vue";
import PreviewStage from "../../../components/settings/PreviewStage.vue";
import { getState, updateState } from "../../../api/pet";
import { getTyped, setSnapRatio, setSnapSleep } from "../../../api/settings";
import {
  deleteCustomAvatar,
  importCustomAvatar,
  readCustomAvatar,
} from "../../../api/customAvatar";
import { listRenderers } from "../../../renderer/factory";
import {
  addCustomConfig,
  loadCustomAvatars,
  listCustomConfigs,
  removeCustomConfig,
  uuidFromFullId,
} from "../../../renderer/customAvatarRegistry";
import { SNAP_SHOW_RATIO_MAX, SNAP_SHOW_RATIO_MIN } from "../../../utils/edgeSnap";
import { pushToast } from "../../../composables/useToast";
import { applyPetState, petStore } from "../../../state/petStore";
import { applySnapshot, settingsStore } from "../../../state/settingsStore";

const { t } = useI18n();

/** 注册表 id → i18n 键（键必须与 `renderer/factory.ts` 的注册表 id 一致）。 */
const PET_META_KEYS: Record<string, { name: string; sub: string; emoji: string }> = {
  code_cat: { name: "avatar.catName", sub: "avatar.catSub", emoji: "🐱" },
  code_dog: { name: "avatar.dogName", sub: "avatar.dogSub", emoji: "🐶" },
};

/** 自定义形象缓存快照（注册表非响应式，经 syncCustomList 同步到响应式副本）。 */
const customList = ref<
  Array<{ id: string; fullId: string; name: string; dataUrl: string }>
>([]);

/** 从注册表同步自定义形象列表（画廊/选中态依赖其刷新）。 */
function syncCustomList(): void {
  customList.value = listCustomConfigs().map((c) => ({
    id: c.id,
    fullId: c.fullId,
    name: c.name,
    dataUrl: c.dataUrl,
  }));
}

/** 选项卡清单 = 内置（注册表派生） + 自定义（注册表合并，卡片直接显示真实缩略图 + 删除）。 */
const OPTIONS = computed(() => [
  ...listRenderers().map((id) => {
    const meta = PET_META_KEYS[id];
    return {
      id,
      fullId: id,
      name: meta ? t(meta.name) : id,
      sub: meta ? t(meta.sub) : t("avatar.builtinFallback"),
      emoji: meta?.emoji ?? "🐾",
      image: undefined,
      deletable: false,
    };
  }),
  ...customList.value.map((c) => ({
    id: c.fullId,
    fullId: c.fullId,
    name: c.name,
    sub: t("avatar.customSub"),
    emoji: undefined,
    image: c.dataUrl,
    deletable: true,
  })),
]);

const savedName = ref(petStore.petName);
let saveTimer: ReturnType<typeof setTimeout> | null = null;

/** 选中形象：import 走导入；其余落库切换。自定义形象微调面板已移除（缩放/锚点滑杆下架）。 */
function selectRenderer(id: string): void {
  if (id === "import") {
    void runImport();
    return;
  }
  const prev = petStore.activeRenderer;
  petStore.activeRenderer = id; // 乐观
  void saveRenderer(id, prev);
}

/** 导入自定义形象：对话框选 PNG → Rust 落库 → 注册表即时插入 → 设为当前 + 打开微调。 */
async function runImport(): Promise<void> {
  // dialog 仅取路径字符串（边界红线：前端不碰文件系统）
  let srcPath: string;
  try {
    const picked = await open({
      multiple: false,
      filters: [{ name: "PNG 透明图", extensions: ["png"] }],
    });
    if (!picked) return; // 用户取消
    srcPath = Array.isArray(picked) ? picked[0] : picked;
  } catch (e) {
    pushToast(e instanceof Error ? e.message : t("avatar.importFailed"), "error");
    return;
  }
  try {
    const meta = await importCustomAvatar(t("avatar.importDefaultName"), srcPath);
    // 即时填充注册表（避免整表重读图片字节）
    let dataUrl = "";
    try {
      dataUrl = await readCustomAvatar(meta.id);
    } catch (e) {
      console.warn("[avatar] 导入后读取图片失败，回退整表刷新", e);
      await loadCustomAvatars().catch(() => undefined);
    }
    const fullId = `custom:${meta.id}`;
    addCustomConfig({
      id: meta.id,
      fullId,
      name: meta.name,
      dataUrl,
      scale: meta.scale,
      anchorX: meta.anchorX,
      anchorY: meta.anchorY,
    });
    syncCustomList();
    pushToast(t("avatar.importOk"));
    // 设为当前形象
    const prev = petStore.activeRenderer;
    petStore.activeRenderer = fullId;
    void saveRenderer(fullId, prev);
  } catch (e) {
    pushToast(e instanceof Error ? e.message : t("avatar.importFailed"), "error");
  }
}

/** 落库形象 id；失败回滚到**切换前的那张**（而非写死回猫）。 */
async function saveRenderer(id: string, prev: string): Promise<void> {
  try {
    await updateState({ activeRenderer: id });
  } catch (e) {
    pushToast(e instanceof Error ? e.message : t("avatar.switchFailed"), "error");
    petStore.activeRenderer = prev;
  }
}

watch(
  () => petStore.petName,
  (v): void => {
    if (v.length > 20) {
      petStore.petName = v.slice(0, 20);
      pushToast(t("avatar.nameMax"), "error");
      return;
    }
    if (v === savedName.value) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout((): void => void saveName(v), 300);
  },
);

async function saveName(v: string): Promise<void> {
  try {
    const dto = await updateState({ petName: v });
    applyPetState(dto);
    savedName.value = dto.petName;
  } catch (e) {
    pushToast(e instanceof Error ? e.message : t("common.saveFailed"), "error");
    petStore.petName = savedName.value;
  }
}

/** 边缘吸附休眠开关（type=3 持久化；保存后刷新快照并 toast）。 */
async function toggleSnapSleep(enabled: boolean): Promise<void> {
  try {
    await setSnapSleep(enabled);
    applySnapshot(await getTyped());
    void emit("settings:snap_sleep:changed", { enabled });
    pushToast(enabled ? t("avatar.snapOn") : t("avatar.snapOff"));
  } catch (e) {
    pushToast(e instanceof Error ? e.message : t("common.saveFailed"), "error");
  }
}

/** 吸附态露出比例（type=3 持久化；拖动结束才落库）。 */
async function changeSnapRatio(v: number): Promise<void> {
  try {
    await setSnapRatio(v);
    applySnapshot(await getTyped());
    void emit("settings:snap_ratio:changed", {
      ratio: settingsStore.snapShowRatio,
    });
  } catch (e: unknown) {
    pushToast(e instanceof Error ? e.message : t("common.saveFailed"), "error");
  }
}

/** 同步当前生效的形象 id（窗口 B 从不整体拉取桌宠状态，否则面板永远显示猫在用）。 */
async function refreshActiveRenderer(): Promise<void> {
  try {
    const dto = await getState();
    if (dto.activeRenderer) {
      petStore.activeRenderer = dto.activeRenderer;
    }
  } catch (e: unknown) {
    console.warn("[avatar] 桌宠状态拉取失败", e);
  }
}

/** 删除自定义形象：确认 → Rust 落库删除 + 注册表即时移除。激活态由 Rust 回退并 emit 事件驱动刷新。 */
async function deleteAvatar(fullId: string): Promise<void> {
  const uuid = uuidFromFullId(fullId);
  if (!uuid) return;
  if (!window.confirm(t("avatar.deleteConfirm"))) return;
  try {
    await deleteCustomAvatar(uuid);
    removeCustomConfig(uuid);
    pushToast(t("avatar.deleteOk"));
    syncCustomList();
  } catch (e: unknown) {
    pushToast(e instanceof Error ? e.message : t("common.saveFailed"), "error");
  }
}

onMounted(async (): Promise<void> => {
  savedName.value = petStore.petName;
  // 先填充自定义形象注册表，再读激活态——否则 custom:<uuid> 在画廊/预览里查不到配置
  await loadCustomAvatars()
    .then(syncCustomList)
    .catch((e: unknown) => console.warn("[avatar] 自定义形象加载失败", e));
  await refreshActiveRenderer();
  if (!settingsStore.snapshot) {
    void getTyped()
      .then(applySnapshot)
      .catch((e: unknown) => {
        console.warn("[avatar] 设置快照拉取失败", e);
      });
  }
});
</script>

<template>
  <div class="avatar-layout">
    <!-- 左：预览 -->
    <AppCard :title="t('avatar.preview')">
      <div class="preview-card">
        <PreviewStage :renderer-id="petStore.activeRenderer" />
        <div class="preview-card__name">{{ petStore.petName }}</div>
        <div class="preview-card__badges">
          <span class="preview-card__badge">{{ t("avatar.badgeJelly") }}</span>
          <span class="preview-card__badge">{{ t("avatar.badgeClickThrough") }}</span>
        </div>
      </div>
    </AppCard>

    <!-- 右上：选择形象 -->
    <AppCard :title="t('avatar.chooseBuiltin')">
      <div class="pet-grid">
        <PetOptionCard
          v-for="o in OPTIONS"
          :key="o.id"
          :name="o.name"
          :sub="o.sub"
          :emoji="o.emoji"
          :image="o.image"
          :deletable="o.deletable"
          :selected="petStore.activeRenderer === o.id"
          @select="selectRenderer(o.id)"
          @delete="deleteAvatar(o.fullId)"
        />
        <PetOptionCard
          :name="t('avatar.importTitle')"
          :sub="t('avatar.importSub')"
          emoji="＋"
          dashed
          @select="selectRenderer('import')"
        />
      </div>
    </AppCard>

    <!-- 右下：窗口与交互 -->
    <AppCard :title="t('avatar.windowInteract')">
      <div class="row2">
        <AppInput
          v-model="petStore.petName"
          :label="t('avatar.petName')"
          :placeholder="t('avatar.petNamePlaceholder')"
          :maxlength="20"
        />
      </div>
      <div class="set-rows">
        <div class="set-row">
          <div>
            <div class="set-row__title">{{ t("avatar.edgeSleep") }}</div>
            <div class="set-row__sub">{{ t("avatar.edgeSleepSub") }}</div>
          </div>
          <AppToggle
            :model-value="settingsStore.snapSleepEnabled"
            @update:model-value="toggleSnapSleep($event)"
          />
        </div>
        <div class="set-row">
          <div>
            <div class="set-row__title">{{ t("avatar.snapRatio") }}</div>
            <div class="set-row__sub">{{ t("avatar.snapRatioSub") }}</div>
          </div>
          <div class="ratio-ctl">
            <input
              class="ratio-ctl__slider"
              type="range"
              :min="SNAP_SHOW_RATIO_MIN"
              :max="SNAP_SHOW_RATIO_MAX"
              step="0.05"
              :value="settingsStore.snapShowRatio"
              :disabled="!settingsStore.snapSleepEnabled"
              @change="changeSnapRatio(Number(($event.target as HTMLInputElement).value))"
            />
            <span class="ratio-ctl__val"
              >{{ Math.round(settingsStore.snapShowRatio * 100) }}%</span
            >
          </div>
        </div>
      </div>
      <HintBox tone="info">
        {{ t("avatar.hint") }}
      </HintBox>
    </AppCard>
  </div>
</template>

<style scoped>
.avatar-layout {
  display: grid;
  grid-template-columns: 296px 1fr;
  gap: 16px;
  align-items: start;
}
.preview-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.preview-card__name {
  font-family: var(--font-disp);
  font-size: 22px;
  color: var(--ink);
  text-align: center;
}
.preview-card__badges {
  display: flex;
  gap: 6px;
  justify-content: center;
  flex-wrap: wrap;
}
.preview-card__badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--bg-soft);
  color: var(--ink-soft);
}
.pet-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}
.row2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  align-items: end;
}
.set-rows {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 12px;
}
.set-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.set-row__title {
  font-size: 13.5px;
  color: var(--ink);
  font-weight: 600;
}
.set-row__sub {
  font-size: 12px;
  color: var(--ink-soft);
}
.ratio-ctl {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ratio-ctl__slider {
  width: 132px;
  accent-color: var(--matcha-deep);
}
.ratio-ctl__val {
  min-width: 38px;
  font-size: 12px;
  color: var(--ink-soft);
  text-align: right;
}
@media (max-width: 1080px) {
  .avatar-layout {
    grid-template-columns: 1fr;
  }
}
</style>

<script setup lang="ts">
/**
 * 桌宠面板（设计方案 §4.1）：名字即改即存（pet_update_state, debounce）+ 形象选择 + 预览。
 * 形象仅 code_cat 可选（code_dog/导入为 Phase 3 占位）；边缘吸附/铃铛开关后端持久化命令缺失，占位。
 */
import { emit } from "@tauri-apps/api/event";
import { onMounted, ref, watch } from "vue";

import AppCard from "../../../components/ui/AppCard.vue";
import AppInput from "../../../components/ui/AppInput.vue";
import AppToggle from "../../../components/ui/AppToggle.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import PetOptionCard from "../../../components/settings/PetOptionCard.vue";
import PreviewStage from "../../../components/settings/PreviewStage.vue";
import { updateState } from "../../../api/pet";
import { getTyped, setSnapRatio, setSnapSleep } from "../../../api/settings";
import { SNAP_SHOW_RATIO_MAX, SNAP_SHOW_RATIO_MIN } from "../../../utils/edgeSnap";
import { pushToast } from "../../../composables/useToast";
import { applyPetState, petStore } from "../../../state/petStore";
import { applySnapshot, settingsStore } from "../../../state/settingsStore";

const OPTIONS = [
  { id: "code_cat", name: "猫猫", sub: "奶油黄", emoji: "🐱", disabled: false },
  { id: "code_dog", name: "狗子", sub: "抹茶绿 · Phase 3", emoji: "🐶", disabled: true },
];

const savedName = ref(petStore.petName);
let saveTimer: ReturnType<typeof setTimeout> | null = null;

function selectRenderer(id: string): void {
  if (id === "code_cat") {
    petStore.activeRenderer = id; // 乐观
    void saveRenderer(id);
  } else {
    pushToast("自定义形象将在 Phase 3 开放", "error");
  }
}

async function saveRenderer(id: string): Promise<void> {
  try {
    await updateState({ activeRenderer: id });
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "切换形象失败", "error");
    petStore.activeRenderer = "code_cat";
  }
}

watch(
  () => petStore.petName,
  (v): void => {
    if (v.length > 20) {
      petStore.petName = v.slice(0, 20);
      pushToast("名字最长 20 字", "error");
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
    pushToast(e instanceof Error ? e.message : "保存失败", "error");
    petStore.petName = savedName.value;
  }
}

/** 边缘吸附休眠开关（type=3 持久化；保存后刷新快照并 toast）。 */
async function toggleSnapSleep(enabled: boolean): Promise<void> {
  try {
    await setSnapSleep(enabled);
    applySnapshot(await getTyped());
    // 同步窗口 A（双窗口独立 webview，store 不共享；A 只在挂载时拉一次快照，
    // 不发事件会让 A 缓存陈旧开关 → 吸附门控失效）
    void emit("settings:snap_sleep:changed", { enabled });
    pushToast(enabled ? "已开启：拖到屏幕边缘会休眠" : "已关闭：拖到边缘不再休眠");
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "保存失败", "error");
  }
}

/**
 * 吸附态露出比例（type=3 持久化；后端 clamp 到 [SNAP_SHOW_RATIO_MIN, MAX]）。
 * 拖动结束（change）才落库，避免拖动过程高频 invoke；落库后回读确认最终值。
 */
async function changeSnapRatio(v: number): Promise<void> {
  try {
    await setSnapRatio(v);
    applySnapshot(await getTyped());
    // 同步窗口 A（休眠中改比例即时生效）
    void emit("settings:snap_ratio:changed", {
      ratio: settingsStore.snapShowRatio,
    });
  } catch (e: unknown) {
    pushToast(e instanceof Error ? e.message : "保存失败", "error");
  }
}

onMounted((): void => {
  savedName.value = petStore.petName;
  // 快照缺省拉取一次（窗口 B 首次打开本面板时 snapshot 可能尚未加载）
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
    <AppCard title="预览">
      <div class="preview-card">
        <PreviewStage :renderer-id="petStore.activeRenderer" />
        <div class="preview-card__name">{{ petStore.petName }}</div>
        <div class="preview-card__badges">
          <span class="preview-card__badge">果冻物理</span>
          <span class="preview-card__badge">视线跟随</span>
          <span class="preview-card__badge">点击穿透</span>
        </div>
      </div>
    </AppCard>

    <!-- 右上：选择形象 -->
    <AppCard title="选择内置形象">
      <div class="pet-grid">
        <PetOptionCard
          v-for="o in OPTIONS"
          :key="o.id"
          :name="o.name"
          :sub="o.sub"
          :emoji="o.emoji"
          :disabled="o.disabled"
          :selected="petStore.activeRenderer === o.id"
          @select="selectRenderer(o.id)"
        />
        <PetOptionCard
          name="导入自定义形象"
          sub="Phase 3 · CustomSpriteRenderer"
          emoji="＋"
          dashed
          @select="selectRenderer('import')"
        />
      </div>
    </AppCard>

    <!-- 右下：窗口与交互 -->
    <AppCard title="窗口与交互">
      <div class="row2">
        <AppInput
          v-model="petStore.petName"
          label="桌宠名字"
          placeholder="喵喵"
          :maxlength="20"
        />
      </div>
      <div class="set-rows">
        <div class="set-row">
          <div>
            <div class="set-row__title">边缘吸附休眠</div>
            <div class="set-row__sub">拖到屏幕边缘即休眠：只露耳朵和眼睛，彻底静默</div>
          </div>
          <AppToggle
            :model-value="settingsStore.snapSleepEnabled"
            @update:model-value="toggleSnapSleep($event)"
          />
        </div>
        <div class="set-row">
          <div>
            <div class="set-row__title">吸附露出比例</div>
            <div class="set-row__sub">贴边时露出形象圆盘的百分比</div>
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
        形象切换与名字已即时生效。休眠期间主动话术、弹窗、好感度衰减与后台日记都会暂停；把桌宠拖离边缘即可唤醒。
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
.readonly-size {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.readonly-size__label {
  font-size: 13px;
  color: var(--ink-soft);
}
.readonly-size__val {
  font-family: Consolas, monospace;
  font-size: 14px;
  color: var(--ink);
  padding: 9px 12px;
  background: var(--bg-deep);
  border-radius: var(--r-sm);
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
  accent-color: var(--mint-strong);
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

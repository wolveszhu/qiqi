<script setup lang="ts">
/**
 * 桌宠面板（设计方案 §4.1）：名字即改即存（pet_update_state, debounce）+ 形象选择 + 预览。
 * 形象仅 code_cat 可选（code_dog/导入为 Phase 3 占位）；边缘吸附/铃铛开关后端持久化命令缺失，占位。
 */
import { onMounted, ref, watch } from "vue";

import AppCard from "../../../components/ui/AppCard.vue";
import AppInput from "../../../components/ui/AppInput.vue";
import AppToggle from "../../../components/ui/AppToggle.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import PetOptionCard from "../../../components/settings/PetOptionCard.vue";
import PreviewStage from "../../../components/settings/PreviewStage.vue";
import { updateState } from "../../../api/pet";
import { pushToast } from "../../../composables/useToast";
import { applyPetState, petStore } from "../../../state/petStore";

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

onMounted((): void => {
  savedName.value = petStore.petName;
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
        <div class="readonly-size">
          <span class="readonly-size__label">窗口尺寸</span>
          <span class="readonly-size__val">400 × 400</span>
        </div>
      </div>
      <div class="set-rows">
        <div class="set-row">
          <div>
            <div class="set-row__title">边缘吸附休眠</div>
            <div class="set-row__sub">拖到屏幕边缘自动休眠</div>
          </div>
          <AppToggle
            :model-value="false"
            :disabled="true"
          />
        </div>
      </div>
      <HintBox tone="info">
        形象切换与名字已即时生效；边缘吸附休眠的后端持久化命令将在后续阶段补齐。
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
@media (max-width: 1080px) {
  .avatar-layout {
    grid-template-columns: 1fr;
  }
}
</style>

<script setup lang="ts">
/**
 * 好感度面板（设计方案 §4.4，基线 §F：禁止播报数值——仅看板内可见）：
 * 左 hero = AffectionRing + LevelPill + 距下档差；左下列「成长路线」地图；
 * 右上 = 加减分规则（静态，已同步新难度规则）；右下 = 近期变动流水。
 *
 * 动效（用户裁决"动效"）：① 分数变动——`petStore.affectionPoints` 变化时用 rAF
 * 做约 600ms ease-out 数字补间（`displayPoints`），圆环随之平滑扫动；② 升级——监听
 * `relationshipLevel` 上升，弹出「升级！」庆祝浮层一次。均用原生 rAF / CSS，不引第三方库。
 */
import { computed, onActivated, onMounted, onUnmounted, ref, watch } from "vue";

import AffectionRing from "../../../components/affection/AffectionRing.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import LevelJourney from "../../../components/affection/LevelJourney.vue";
import LevelPill from "../../../components/affection/LevelPill.vue";
import LogItem from "../../../components/affection/LogItem.vue";
import { getState, listAffectionLogs } from "../../../api/pet";
import { applyPetState, petStore, setPetLogs } from "../../../state/petStore";
import { levelLabel, nextLevelInfo } from "../../../utils/affection";
import type { LogItem as LogItemDto } from "../../../api/types";

/** 动画用展示分（tween 后的整数），避免数字硬跳。 */
const displayPoints = ref(petStore.affectionPoints);
let rafId = 0;

/** 成长路线卡片折叠态（默认展开）。 */
const journeyCollapsed = ref(false);

/** 数值补间：ease-out cubic，约 600ms。 */
function tweenTo(target: number): void {
  cancelAnimationFrame(rafId);
  const start = displayPoints.value;
  const delta = target - start;
  if (Math.abs(delta) < 1) {
    displayPoints.value = target;
    return;
  }
  const dur = 600;
  const t0 = performance.now();
  const tick = (now: number): void => {
    const k = Math.min(1, (now - t0) / dur);
    const e = 1 - Math.pow(1 - k, 3);
    displayPoints.value = start + delta * e;
    if (k < 1) rafId = requestAnimationFrame(tick);
    else displayPoints.value = target;
  };
  rafId = requestAnimationFrame(tick);
}

watch(
  () => petStore.affectionPoints,
  (v) => tweenTo(v),
);

/** 距下档差（随动画分平滑变化；level 即时切换）。 */
const nextInfo = computed(() =>
  nextLevelInfo(petStore.relationshipLevel, Math.round(displayPoints.value)),
);

// 升级庆祝浮层
const celebrate = ref(false);
let celebrateTimer: number | undefined;
const lastKnownLevel = ref(petStore.relationshipLevel);
const loadedLevel = ref(false);
watch(
  () => petStore.relationshipLevel,
  (lv) => {
    // 面板刚加载时把占位等级(1)刷成真实值，不算"升级"，不弹庆祝
    if (!loadedLevel.value) return;
    if (lv > lastKnownLevel.value) {
      celebrate.value = false;
      // 下一帧再置 true，确保 transition 重新触发
      requestAnimationFrame(() => {
        celebrate.value = true;
      });
      window.clearTimeout(celebrateTimer);
      celebrateTimer = window.setTimeout(() => {
        celebrate.value = false;
      }, 1800);
    }
    lastKnownLevel.value = lv;
  },
);

const RULES: Array<{ title: string; delta: string; plus: boolean }> = [
  { title: "每日打卡", delta: "+10", plus: true },
  { title: "抚摸（日上限 20）", delta: "+1", plus: true },
  { title: "深度对话（日上限·递减）", delta: "+5→0", plus: true },
  { title: "情感共鸣（日上限·递减）", delta: "+10~+20", plus: true },
  { title: "超 3 天未互动（止于等级下限）", delta: "−5", plus: false },
  { title: "触犯禁忌", delta: "−10~−30", plus: false },
];

/**
 * 拉取好感度数据（总点数 + 流水）。
 *
 * 窗口 B 不运行 `useProactive`（那套只在窗口 A `PetWindow` 挂载），本面板必须自行拉一次
 * `pet_get_state`，否则 `petStore` 停留在初始 0 分——圆环与「距下档差值」都显示不出真实点数。
 * 面板位于 `<KeepAlive>` 内（切 Tab 不卸载），故同时挂 `onActivated`：每次切回本 Tab 刷新，
 * 避免窗口 A 结算后看板仍是旧值。
 */
async function load(): Promise<void> {
  try {
    applyPetState(await getState());
    // 真实等级落库后再门控庆祝：此刻的占位→真实值跳变不算升级
    lastKnownLevel.value = petStore.relationshipLevel;
    loadedLevel.value = true;
  } catch (e) {
    console.warn("[affection] 状态拉取失败", e);
  }
  try {
    setPetLogs((await listAffectionLogs(50)) as LogItemDto[]);
  } catch (e) {
    console.warn("[affection] 流水拉取失败", e);
  }
}

onMounted(load);
onActivated(load);
onUnmounted(() => {
  cancelAnimationFrame(rafId);
  window.clearTimeout(celebrateTimer);
});
</script>

<template>
  <div class="aff-layout">
    <!-- 左列：等级 hero + 成长路线地图 -->
    <div class="aff-col">
      <AppCard title="好感度">
        <div class="aff-hero">
          <AffectionRing :points="Math.round(displayPoints)" />
          <LevelPill :level="petStore.relationshipLevel" />
          <div class="aff-next">
            距离下一级还差 <b>{{ nextInfo.diff }}</b> 分 · {{ nextInfo.sub }}
          </div>
          <!-- <div class="aff-badges">
            <span class="aff-badge">弹性衰减已启用</span>
            <span class="aff-badge">禁止播报数值</span>
          </div> -->
        </div>

        <Transition name="pop">
          <div
            v-if="celebrate"
            class="aff-celebrate"
          >
            🎉 升级！Lv.{{ petStore.relationshipLevel }}
            {{ levelLabel(petStore.relationshipLevel) }}
          </div>
        </Transition>
      </AppCard>

      <AppCard
        v-model:collapsed="journeyCollapsed"
        title="成长路线"
        collapsible
      >
        <template #collapsed>
          <LevelPill :level="petStore.relationshipLevel" />
          <span class="journey-mini__text">
            Lv.{{ petStore.relationshipLevel }}
            {{ levelLabel(petStore.relationshipLevel) }} · 当前
            {{ Math.round(displayPoints) }} 分（进行中）
          </span>
        </template>
        <LevelJourney
          :points="Math.round(displayPoints)"
          :level="petStore.relationshipLevel"
        />
      </AppCard>
    </div>

    <!-- 右列：规则 + 流水 -->
    <div class="aff-col aff-col--right">
      <AppCard title="加减分规则">
        <div class="rule-list">
          <div
            v-for="r in RULES"
            :key="r.title"
            class="set-row"
          >
            <span class="set-row__title">{{ r.title }}</span>
            <span
              class="delta"
              :class="r.plus ? 'plus' : 'minus'"
              >{{ r.delta }}</span
            >
          </div>
        </div>
      </AppCard>

      <AppCard title="近期变动">
        <div
          v-if="petStore.logs.length"
          class="log-list"
        >
          <LogItem
            v-for="log in petStore.logs"
            :key="log.id"
            :log="log"
          />
        </div>
        <HintBox v-else> 还没有流水，去摸摸她吧。 </HintBox>
      </AppCard>
    </div>
  </div>
</template>

<style scoped>
.aff-layout {
  display: grid;
  grid-template-columns: 320px 1fr;
  gap: 16px;
  align-items: start;
}
.aff-col {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

.aff-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
}
.aff-next {
  font-size: 13px;
  color: var(--ink-soft);
  text-align: center;
}
.aff-next b {
  color: var(--peach);
}
.aff-badges {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  justify-content: center;
}
.aff-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--bg-deep);
  color: var(--ink-soft);
}

/* 升级庆祝浮层 */
.aff-celebrate {
  position: absolute;
  top: 10px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 5;
  padding: 6px 16px;
  border-radius: 999px;
  background: var(--peach);
  color: #fff;
  font-family: var(--font-disp);
  font-size: 14px;
  font-weight: 600;
  white-space: nowrap;
  box-shadow: 0 4px 14px rgba(224, 142, 109, 0.4);
}
.pop-enter-active,
.pop-leave-active {
  transition: all 0.4s cubic-bezier(0.22, 1, 0.36, 1);
}
.pop-enter-from,
.pop-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-8px) scale(0.9);
}

.rule-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
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
}
.delta {
  font-family: Consolas, monospace;
  font-size: 13px;
  font-weight: 700;
}
.delta.plus {
  color: var(--matcha-deep);
}
.delta.minus {
  color: var(--peach);
}
.log-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

/* 成长路线折叠态摘要 */
.journey-mini__text {
  font-size: 13px;
  color: var(--ink-soft);
}
.journey-mini__text b {
  color: var(--peach);
}

@media (max-width: 1080px) {
  .aff-layout {
    grid-template-columns: 1fr;
  }
}
</style>

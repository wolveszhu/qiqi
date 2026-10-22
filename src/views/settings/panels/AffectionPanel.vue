<script setup lang="ts">
/**
 * 好感度面板（设计方案 §4.4，基线 §F：禁止播报数值——仅看板内可见）：
 * 左 hero = AffectionRing + LevelPill + 距下档差；右上 = 加减分规则（静态）；
 * 右下 = 近期变动流水（pet_list_affection_logs）。数值/差值是唯一暴露点，绝不进对话气泡。
 */
import { computed, onMounted } from "vue";

import AffectionRing from "../../../components/affection/AffectionRing.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import LevelPill from "../../../components/affection/LevelPill.vue";
import LogItem from "../../../components/affection/LogItem.vue";
import { listAffectionLogs } from "../../../api/pet";
import { petStore, setPetLogs } from "../../../state/petStore";
import type { LogItem as LogItemDto } from "../../../api/types";

/** 等级阈值（展示近似：0–1000 四等分布；与 C5 等级映射视觉对齐）。 */
const LEVEL_THRESHOLDS = [0, 250, 500, 750, 1000];
/** 下档表现文案（情绪化，非数值）。 */
const LEVEL_SUB = [
  "",
  "刚认识，有点拘谨",
  "熟悉了，会主动搭话",
  "亲密，会撒娇会想念",
  "羁绊，形影不离",
];

const nextInfo = computed(() => {
  const lvl = petStore.relationshipLevel;
  if (lvl >= 4) return { diff: 0, sub: "已抵达最高羁绊" };
  const next = LEVEL_THRESHOLDS[lvl];
  return {
    diff: Math.max(0, next - petStore.affectionPoints),
    sub: LEVEL_SUB[lvl + 1] ?? "",
  };
});

const RULES: Array<{ title: string; delta: string; plus: boolean }> = [
  { title: "每日打卡", delta: "+10", plus: true },
  { title: "抚摸（日上限 20）", delta: "+1", plus: true },
  { title: "深度对话 · 共鸣", delta: "+5~+20", plus: true },
  { title: "超 3 天未互动（止于等级下限）", delta: "−5", plus: false },
  { title: "触犯禁忌", delta: "−10~−30", plus: false },
];

onMounted(async (): Promise<void> => {
  try {
    setPetLogs((await listAffectionLogs(50)) as LogItemDto[]);
  } catch (e) {
    console.warn("[affection] 流水拉取失败", e);
  }
});
</script>

<template>
  <div class="aff-layout">
    <!-- 左：等级 hero -->
    <AppCard title="好感度">
      <div class="aff-hero">
        <AffectionRing :points="petStore.affectionPoints" />
        <LevelPill :level="petStore.relationshipLevel" />
        <div class="aff-next">
          距离下一级还差 <b>{{ nextInfo.diff }}</b> 分 · {{ nextInfo.sub }}
        </div>
        <div class="aff-badges">
          <span class="aff-badge">弹性衰减已启用</span>
          <span class="aff-badge">禁止播报数值</span>
        </div>
      </div>
    </AppCard>

    <!-- 右上：规则卡 -->
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

    <!-- 右下：流水卡 -->
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
</template>

<style scoped>
.aff-layout {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 16px;
  align-items: start;
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
@media (max-width: 1080px) {
  .aff-layout {
    grid-template-columns: 1fr;
  }
}
</style>

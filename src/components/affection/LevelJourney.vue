<script setup lang="ts">
/**
 * 等级成长地图（好感度面板改造：用户裁决"等级成长地图 + 动效"）。
 *
 * 把"陌生 / 熟人 / 亲密 / 羁绊"四档做成一条可见的成长路线：
 * - 已达成：抹茶实心节点 + 对勾 + 满进度条
 * - 当前（进行中）：蜜桃高亮描边 + 脉冲光晕 + 实时档内进度
 * - 未解锁：灰底锁形 + 零进度
 *
 * 纯展示，props 由 `AffectionPanel` 注入（`petStore` 的 points / level）。
 * 阈值与 `utils/affection` 单一真源对齐，不在此重复魔法数。
 */
import { computed } from "vue";

import { LEVEL_NAMES, LEVEL_THRESHOLDS } from "../../utils/affection";

const props = defineProps<{ points: number; level: number }>();

type StepState = "achieved" | "current" | "locked";

interface Step {
  idx: number;
  name: string;
  /** 进入该等级所需分数（门槛）。 */
  require: number;
  state: StepState;
  /** 档内进度 0–1（achieved 满，locked 空，current 实时）。 */
  progress: number;
}

const steps = computed<Step[]>(() => {
  const out: Step[] = [];
  for (let i = 1; i <= 4; i++) {
    const lower = LEVEL_THRESHOLDS[i - 1];
    const upper = LEVEL_THRESHOLDS[i];
    const span = Math.max(1, upper - lower);
    let progress = 0;
    if (i < props.level) progress = 1;
    else if (i === props.level) {
      progress = Math.max(0, Math.min(1, (props.points - lower) / span));
    }
    out.push({
      idx: i,
      name: LEVEL_NAMES[i] ?? `Lv.${i}`,
      require: lower,
      state: i < props.level ? "achieved" : i === props.level ? "current" : "locked",
      progress,
    });
  }
  return out;
});
</script>

<template>
  <div class="journey">
    <div
      v-for="(s, i) in steps"
      :key="s.idx"
      class="jstep"
      :class="[`is-${s.state}`, { 'is-last': i === steps.length - 1 }]"
    >
      <!-- 左侧铁轨：节点 + 贯穿连接线（成长路线观感） -->
      <div class="jstep__rail">
        <span class="jstep__node">
          <span
            v-if="s.state === 'achieved'"
            class="jstep__mark"
            >✓</span
          >
          <span
            v-else-if="s.state === 'locked'"
            class="jstep__mark"
            >🔒</span
          >
        </span>
      </div>

      <!-- 右侧内容 -->
      <div class="jstep__body">
        <div class="jstep__head">
          <span class="jstep__name">Lv.{{ s.idx }} {{ s.name }}</span>
          <span class="jstep__req">
            {{ s.require === 0 ? "起点" : `${s.require} 分解锁` }}
          </span>
        </div>

        <div class="jstep__bar">
          <span
            class="jstep__fill"
            :style="{ width: `${s.progress * 100}%` }"
          ></span>
        </div>

        <div class="jstep__tag">
          <span v-if="s.state === 'achieved'">已达成</span>
          <span v-else-if="s.state === 'current'">进行中</span>
          <span v-else>未解锁</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.journey {
  display: flex;
  flex-direction: column;
  gap: 4px;
  --c-matcha: #5d8541;
  --c-peach: #e08e6d;
  --c-ink: #3a332a;
  --c-soft: #8a7f6e;
}

.jstep {
  display: flex;
  gap: 12px;
  padding: 4px 0;
}

/* 铁轨：节点 + 向下贯穿连接线 */
.jstep__rail {
  position: relative;
  width: 26px;
  flex: 0 0 26px;
  display: flex;
  justify-content: center;
}
.jstep__rail::after {
  content: "";
  position: absolute;
  top: 26px;
  bottom: -4px;
  left: 50%;
  width: 2px;
  transform: translateX(-50%);
  background: linear-gradient(var(--c-matcha), var(--c-peach));
  opacity: 0.35;
}
.jstep.is-last .jstep__rail::after {
  display: none;
}

.jstep__node {
  position: relative;
  z-index: 1;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  background: var(--bg-window);
  border: 2px solid var(--bg-deep);
  color: var(--c-soft);
  transition:
    transform 0.25s ease,
    box-shadow 0.25s ease;
}

.jstep__body {
  flex: 1 1 auto;
  min-width: 0;
  background: var(--bg-window);
  border: 1px solid color-mix(in srgb, var(--c-soft) 22%, transparent);
  border-radius: 12px;
  padding: 9px 12px;
  box-shadow: 0 1px 2px rgba(58, 51, 42, 0.06);
}

.jstep__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}
.jstep__name {
  font-family: var(--font-disp);
  font-size: 15px;
  font-weight: 600;
  color: var(--c-ink);
}
.jstep__req {
  font-size: 11.5px;
  color: var(--c-soft);
}

.jstep__bar {
  margin: 7px 0 5px;
  height: 7px;
  border-radius: 999px;
  background: var(--bg-deep);
  overflow: hidden;
}
.jstep__fill {
  display: block;
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--c-matcha), #8fb46a);
  transition: width 0.6s cubic-bezier(0.22, 1, 0.36, 1);
}

.jstep__tag {
  font-size: 11px;
  color: var(--c-soft);
}

/* 三态样式 */
.jstep.is-achieved .jstep__node {
  background: var(--c-matcha);
  border-color: var(--c-matcha);
  color: #fff;
}
.jstep.is-achieved .jstep__fill {
  background: linear-gradient(90deg, var(--c-matcha), #8fb46a);
}

.jstep.is-current .jstep__node {
  background: var(--c-peach);
  border-color: var(--c-peach);
  color: #fff;
  animation: jpulse 2s ease-out infinite;
}
.jstep.is-current .jstep__body {
  border-color: color-mix(in srgb, var(--c-peach) 55%, transparent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--c-peach) 30%, transparent);
}
.jstep.is-current .jstep__fill {
  background: linear-gradient(90deg, var(--c-peach), #f0b194);
}
.jstep.is-current .jstep__name {
  color: var(--c-peach);
}

.jstep.is-locked .jstep__node {
  background: var(--bg-deep);
  border-color: var(--bg-deep);
  color: var(--c-soft);
}
.jstep.is-locked .jstep__name {
  color: var(--c-soft);
}
.jstep.is-locked .jstep__fill {
  background: var(--c-soft);
  opacity: 0.4;
}

@keyframes jpulse {
  0% {
    box-shadow: 0 0 0 0 rgba(224, 142, 109, 0.45);
  }
  70% {
    box-shadow: 0 0 0 11px rgba(224, 142, 109, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(224, 142, 109, 0);
  }
}

/* 窄屏：卡片内 padding 收紧，铁轨略缩 */
@media (max-width: 1080px) {
  .jstep__body {
    padding: 8px 10px;
  }
}
</style>

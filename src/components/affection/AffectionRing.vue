<script setup lang="ts">
/**
 * 好感度圆环（设计方案 §5.2 / §4.4）：conic-gradient 进度（--peach），内白圆居中数字。
 * 纯展示，points 由父级注入（0–1000）。
 */
import { computed } from "vue";

import { AFFECTION_MAX, ringAngle } from "../../utils/affection";

const props = defineProps<{ points: number }>();

// 进度/角度计算抽纯函数（utils/affection；边界由 F2.2 单测固化）
const angle = computed(() => ringAngle(props.points));
</script>

<template>
  <div
    class="aff-ring"
    :style="{
      background: `conic-gradient(var(--peach) ${angle}deg, #EDE2CB ${angle}deg 360deg)`,
    }"
    role="img"
    :aria-label="`好感度 ${points} / ${AFFECTION_MAX}`"
  >
    <div class="aff-ring__inner">
      <span class="aff-ring__num">{{ points }}</span>
      <span class="aff-ring__of">of 1000</span>
    </div>
  </div>
</template>

<style scoped>
.aff-ring {
  position: relative;
  width: 158px;
  height: 158px;
  border-radius: 50%;
}
.aff-ring__inner {
  position: absolute;
  inset: 13px;
  background: var(--bg-window);
  border-radius: 50%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}
.aff-ring__num {
  font-family: var(--font-disp);
  font-size: 40px;
  color: var(--ink);
  line-height: 1;
}
.aff-ring__of {
  font-size: 10.5px;
  color: var(--ink-soft);
  margin-top: 2px;
}
</style>

<script setup lang="ts">
/**
 * 内置额度进度条（设计方案 §5.2 / §4.2）：轨道 --bg-deep，填充 butter→peach 渐变，
 * 宽度=剩余比例。ui/ 红线：纯展示，ratio 由父级注入（0–1）。
 */
defineProps<{ ratio: number }>();
</script>

<template>
  <div
    class="quota-bar"
    role="progressbar"
    :aria-valuenow="Math.round(Math.max(0, Math.min(1, ratio)) * 100)"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <div
      class="quota-bar__fill"
      :style="{ width: `${Math.max(0, Math.min(1, ratio)) * 100}%` }"
    ></div>
  </div>
</template>

<style scoped>
.quota-bar {
  height: 10px;
  border-radius: 999px;
  background: var(--bg-deep);
  overflow: hidden;
}
.quota-bar__fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--butter), var(--peach));
  transition: width var(--dur-slow) var(--ease-out);
}
</style>

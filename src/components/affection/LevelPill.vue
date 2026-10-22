<script setup lang="ts">
/**
 * 等级胶囊（设计方案 §5.2 / §4.4）：深棕底白字，level → 文案映射（陌生/熟人/亲密/羁绊）。
 * 纯展示。
 */
import { computed } from "vue";

const props = defineProps<{ level: number }>();

// baseline §F 等级命名（1–4）；未知等级回退 "Lv.N"。
const NAME = ["", "陌生", "熟人", "亲密", "羁绊"];

const name = computed(() => NAME[props.level] ?? `Lv.${props.level}`);
</script>

<template>
  <span class="lv-pill">Lv.{{ level }} {{ name }}</span>
</template>

<style scoped>
.lv-pill {
  display: inline-flex;
  align-items: center;
  padding: 5px 14px;
  background: var(--ink);
  color: var(--bg-window);
  border-radius: 999px;
  font-family: var(--font-disp);
  font-size: 17px;
  letter-spacing: 2px;
  box-shadow: var(--shadow-card);
}
</style>

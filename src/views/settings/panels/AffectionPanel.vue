<script setup lang="ts">
/**
 * 好感度面板（设计方案 §4.4）：圆环 + 等级 + 流水。
 * TODO(D2): AffectionRing（conic-gradient）+ LevelPill + LogItem 列表。
 */

import AppCard from "../../../components/ui/AppCard.vue";
import { petStore } from "../../../state/petStore";
</script>

<template>
  <AppCard title="好感度">
    <p class="affection-panel__line">
      Lv.{{ petStore.relationshipLevel }} · {{ petStore.affectionPoints }} / 1000
    </p>
    <!-- TODO(D2): AffectionRing + LevelPill + LogItem 列表 -->
  </AppCard>
</template>

<style scoped>
.affection-panel__line {
  font-size: 14px;
  color: var(--ink);
}
</style>

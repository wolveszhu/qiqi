<script setup lang="ts">
/**
 * 记忆面板（设计方案 §4.5）：日记列表 + facts 标签云。
 * TODO(D3): DiaryCard/FactChip 实装（依赖 C7 后端）；当前仅空态。
 */

import AppCard from "../../../components/ui/AppCard.vue";
import HintBox from "../../../components/ui/HintBox.vue";
</script>

<template>
  <AppCard title="记忆">
    <HintBox>今天还没有日记，和七七说说话吧。</HintBox>
    <!-- TODO(D3): 日记列表 + facts 标签云 -->
  </AppCard>
</template>

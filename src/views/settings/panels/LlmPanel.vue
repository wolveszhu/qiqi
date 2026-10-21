<script setup lang="ts">
/**
 * 大模型面板（设计方案 §4.2 + 阶段3补充选型方案）：三档配置 + Key + 服务商预设。
 * TODO(D2): SegmentedControl 三档切换、ProviderChip 组（必选/可选+成本档徽标）、
 * 必选约束引导 HintBox、fallback 回落提示、计费说明入口；显式保存 → settings_save_typed。
 */

import AppCard from "../../../components/ui/AppCard.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import { settingsStore } from "../../../state/settingsStore";
</script>

<template>
  <AppCard title="大模型配置">
    <HintBox tone="warn">
      至少配置 DeepSeek 或通义千问之一，即可独立支撑「主对话 + 记忆沉淀」全流程。
    </HintBox>
    <p
      v-if="!settingsStore.snapshot"
      class="llm-panel__empty"
    >
      正在读取配置…
    </p>
    <!-- TODO(D2): 三档配置表单 + 保存 -->
  </AppCard>
</template>

<style scoped>
.llm-panel__empty {
  font-size: 13px;
  color: var(--ink-soft);
}
</style>

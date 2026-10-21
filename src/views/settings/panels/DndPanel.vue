<script setup lang="ts">
/**
 * 勿扰与频率面板（设计方案 §4.3）：勿扰开关 + 多段时段 + 主动频率档位。
 * TODO(D2): TimeChip 多段增删（起止校验）+ SegmentedControl 三档（1-2/4-6/10+ 次/天）。
 */

import AppCard from "../../../components/ui/AppCard.vue";
import AppToggle from "../../../components/ui/AppToggle.vue";
import { settingsStore } from "../../../state/settingsStore";
import { ref } from "vue";

const enabled = ref(settingsStore.snapshot?.dnd.enabled ?? false);
</script>

<template>
  <AppCard title="勿扰与主动频率">
    <AppToggle
      v-model="enabled"
      label="勿扰模式（静默主动互动）"
    />
    <!-- TODO(D2): 多段时段 TimeChip + 频率 SegmentedControl -->
  </AppCard>
</template>

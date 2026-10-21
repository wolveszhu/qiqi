<script setup lang="ts">
/**
 * 桌宠面板（设计方案 §4.1）：名字 + 渲染器选择 + 预览。
 * TODO(D2): PetOptionCard×2（code_cat/code_dog 禁用态）+ PreviewStage 独立实例实装。
 */

import AppCard from "../../../components/ui/AppCard.vue";
import AppInput from "../../../components/ui/AppInput.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import { petStore } from "../../../state/petStore";
</script>

<template>
  <AppCard title="桌宠">
    <AppInput
      v-model="petStore.petName"
      label="桌宠名字"
      placeholder="喵喵"
    />
    <HintBox>更换形象（自定义形象）将在后续版本开放。</HintBox>
    <!-- TODO(D2): PetOptionCard ×2 + PreviewStage -->
  </AppCard>
</template>

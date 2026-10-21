<script setup lang="ts">
/**
 * 通用卡片（设计方案 §5.1）：暖奶油底 + 细描边 + 纸感阴影。
 * 标题/副标题插槽化；零业务依赖。
 */

defineProps<{ title?: string }>();
</script>

<template>
  <section class="app-card">
    <h3
      v-if="title"
      class="app-card__title"
    >
      {{ title }}
    </h3>
    <div class="app-card__body">
      <slot></slot>
    </div>
  </section>
</template>

<style scoped>
.app-card {
  background: var(--bg-window);
  border: 1px solid var(--line);
  border-radius: var(--r-md);
  box-shadow: var(--shadow-card);
  padding: 18px 20px;
}

.app-card__title {
  font-family: var(--font-body);
  font-size: 15px;
  font-weight: 700;
  color: var(--ink);
  margin-bottom: 12px;
}

.app-card__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
</style>

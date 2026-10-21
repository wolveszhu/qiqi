<script setup lang="ts">
/**
 * 设置窗侧导航（设计方案 §3/§4）：五个 Tab，选中项由父级受控。
 */

withDefaults(
  defineProps<{ active?: "avatar" | "llm" | "dnd" | "affection" | "memory" }>(),
  { active: "avatar" },
);

const emit = defineEmits<{
  (e: "change", key: "avatar" | "llm" | "dnd" | "affection" | "memory"): void;
}>();

const items = [
  { key: "avatar", label: "桌宠" },
  { key: "llm", label: "大模型" },
  { key: "dnd", label: "勿扰与频率" },
  { key: "affection", label: "好感度" },
  { key: "memory", label: "记忆" },
] as const;
</script>

<template>
  <nav class="sidenav">
    <button
      v-for="item in items"
      :key="item.key"
      class="sidenav__item"
      :class="{ 'is-active': item.key === active }"
      @click="emit('change', item.key)"
    >
      {{ item.label }}
    </button>
  </nav>
</template>

<style scoped>
.sidenav {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 168px;
  padding: 16px 12px;
  background: var(--bg-soft);
  border-right: 1px solid var(--line);
}

.sidenav__item {
  border: none;
  background: none;
  text-align: left;
  padding: 10px 14px;
  border-radius: var(--r-sm);
  font-size: 14px;
  color: var(--ink-soft);
  cursor: pointer;
}

.sidenav__item.is-active {
  background: var(--bg-window);
  color: var(--matcha-deep);
  font-weight: 700;
  box-shadow: var(--shadow-card);
}
</style>

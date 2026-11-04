<script setup lang="ts">
/**
 * 设置窗侧导航（设计方案 §3/§4）：六个 Tab，选中项由父级受控。
 */

withDefaults(
  defineProps<{
    active?: "chat" | "avatar" | "llm" | "dnd" | "affection" | "memory" | "about";
    hasUpdate?: boolean;
  }>(),
  { active: "avatar", hasUpdate: false },
);

const emit = defineEmits<{
  (
    e: "change",
    key: "chat" | "avatar" | "llm" | "dnd" | "affection" | "memory" | "about",
  ): void;
}>();

const items = [
  { key: "chat", label: "对话" },
  { key: "avatar", label: "桌宠" },
  { key: "llm", label: "大模型" },
  { key: "dnd", label: "勿扰与频率" },
  { key: "affection", label: "好感度" },
  { key: "memory", label: "记忆" },
  { key: "about", label: "关于" },
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
      <span class="sidenav__label">{{ item.label }}</span>
      <span
        v-if="item.key === 'about' && hasUpdate"
        class="sidenav__badge"
        aria-label="有可用更新"
      ></span>
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.sidenav__label {
  flex: 1;
}

.sidenav__badge {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--peach);
  flex-shrink: 0;
}

.sidenav__item.is-active {
  background: var(--bg-window);
  color: var(--matcha-deep);
  font-weight: 700;
  box-shadow: var(--shadow-card);
}
</style>

<script setup lang="ts">
/**
 * 设置窗侧导航（设计方案 §3/§4）：八个 Tab，选中项由父级受控。
 */

import { useI18n } from "vue-i18n";

withDefaults(
  defineProps<{
    active?:
      "chat" | "avatar" | "llm" | "dnd" | "affection" | "memory" | "about" | "general";
    hasUpdate?: boolean;
  }>(),
  { active: "avatar", hasUpdate: false },
);

const emit = defineEmits<{
  (
    e: "change",
    key: "chat" | "avatar" | "llm" | "dnd" | "affection" | "memory" | "about" | "general",
  ): void;
}>();

const { t } = useI18n();

const items = [
  { key: "chat" },
  { key: "avatar" },
  { key: "llm" },
  { key: "dnd" },
  { key: "affection" },
  { key: "memory" },
  { key: "general" },
  { key: "about" },
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
      <span class="sidenav__label">{{ t(`sideNav.${item.key}`) }}</span>
      <span
        v-if="item.key === 'about' && hasUpdate"
        class="sidenav__badge"
        :aria-label="t('sideNav.updateAvailable')"
      ></span>
    </button>
    <!-- 底部广告位由父级注入（slot 保持本组件零业务依赖） -->
    <div class="sidenav__footer">
      <slot name="footer"></slot>
    </div>
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

/* margin-top:auto 把广告卡贴到侧栏底部；无 slot 内容时为空容器、不占视觉空间 */
.sidenav__footer {
  margin-top: auto;
  padding-top: 12px;
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

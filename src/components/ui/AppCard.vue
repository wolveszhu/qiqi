<script setup lang="ts">
/**
 * 通用卡片（设计方案 §5.1）：暖奶油底 + 细描边 + 纸感阴影。
 * 标题/副标题插槽化；零业务依赖。
 *
 * 可选折叠：`collapsible` 开启后标题行变为可点击，右侧出现箭头；
 * 折叠态通过 `#collapsed` 插槽展示精简摘要，`#extra` 插槽可常驻标题右侧。
 */
import { computed } from "vue";

const props = defineProps<{
  title?: string;
  /** 是否开启折叠（默认关）。 */
  collapsible?: boolean;
  /** 折叠状态，支持 v-model:collapsed。 */
  collapsed?: boolean;
}>();

const emit = defineEmits<{ "update:collapsed": [boolean] }>();

const isCollapsed = computed({
  get: () => props.collapsed ?? false,
  set: (v) => emit("update:collapsed", v),
});

/** 点击标题行切换折叠态（仅 collapsible 生效）。 */
function toggle(): void {
  if (!props.collapsible) return;
  isCollapsed.value = !isCollapsed.value;
}
</script>

<template>
  <section class="app-card">
    <div
      class="app-card__head"
      :class="{ 'is-clickable': collapsible }"
      @click="toggle"
    >
      <h3
        v-if="title"
        class="app-card__title"
      >
        {{ title }}
      </h3>
      <div class="app-card__extra">
        <slot name="extra"></slot>
        <span
          v-if="collapsible"
          class="app-card__chev"
          :class="{ 'is-open': !isCollapsed }"
          aria-hidden="true"
          >▾</span
        >
      </div>
    </div>

    <div
      v-show="!isCollapsed"
      class="app-card__body"
    >
      <slot></slot>
    </div>

    <div
      v-if="isCollapsed"
      class="app-card__collapsed"
    >
      <slot name="collapsed">
        <slot></slot>
      </slot>
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

.app-card__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.app-card__head.is-clickable {
  cursor: pointer;
  user-select: none;
}
.app-card__head.is-clickable:hover .app-card__title {
  color: var(--peach);
}

.app-card__title {
  font-family: var(--font-body);
  font-size: 15px;
  font-weight: 700;
  color: var(--ink);
  margin-bottom: 0;
}

.app-card__extra {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}
.app-card__chev {
  font-size: 13px;
  line-height: 1;
  color: var(--ink-soft);
  transition: transform 0.25s cubic-bezier(0.22, 1, 0.36, 1);
}
.app-card__chev.is-open {
  transform: rotate(180deg);
}

.app-card__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.app-card__collapsed {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}
</style>

<script setup lang="ts">
/**
 * 时段胶囊（设计方案 §5.1 / §4.3）：mono 展示 start–end + 内嵌时钟图标；
 * hover 显示 ✕ 删除；可编辑变体（editable）下内联两个 input type=time。
 * ui/ 红线：零业务依赖，仅受控 range 驱动 + 删除/更新事件。
 */
import { ref } from "vue";

interface Range {
  start: string;
  end: string;
}

const props = defineProps<{ range: Range; editable?: boolean }>();
const emit = defineEmits<{
  (e: "remove"): void;
  (e: "update", range: Range): void;
}>();

const start = ref(props.range.start);
const end = ref(props.range.end);
const editing = ref(false);

function commit(): void {
  editing.value = false;
  emit("update", { start: start.value, end: end.value });
}

function onRemove(): void {
  emit("remove");
}

function enterEdit(): void {
  if (!props.editable) return;
  start.value = props.range.start;
  end.value = props.range.end;
  editing.value = true;
}
</script>

<template>
  <span
    class="time-chip"
    :class="{ 'is-editing': editing }"
    @dblclick="enterEdit"
  >
    <svg
      v-if="!editing"
      class="time-chip__icon"
      viewBox="0 0 24 24"
      width="13"
      height="13"
      aria-hidden="true"
    >
      <circle
        cx="12"
        cy="12"
        r="9"
        fill="none"
        stroke="var(--matcha-deep)"
        stroke-width="2"
      />
      <path
        d="M12 7v5l3 2"
        fill="none"
        stroke="var(--matcha-deep)"
        stroke-width="2"
        stroke-linecap="round"
      />
    </svg>

    <template v-if="!editing">
      <span class="time-chip__text">{{ range.start }} – {{ range.end }}</span>
      <button
        v-if="editable"
        class="time-chip__del"
        type="button"
        aria-label="删除时段"
        @click.stop="onRemove"
      >
        ✕
      </button>
    </template>

    <template v-else>
      <input
        v-model="start"
        class="time-chip__input"
        type="time"
        aria-label="开始时间"
        @change="commit"
      />
      <span class="time-chip__dash">–</span>
      <input
        v-model="end"
        class="time-chip__input"
        type="time"
        aria-label="结束时间"
        @change="commit"
      />
    </template>
  </span>
</template>

<style scoped>
.time-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  background: var(--bg-window);
  border: 1px solid var(--line);
  border-radius: 11px;
  font-family: Consolas, monospace;
  font-size: 13px;
  font-weight: 700;
  color: var(--ink);
}
.time-chip.is-editing {
  background: var(--bg-soft);
}
.time-chip__icon {
  flex: none;
}
.time-chip__text {
  white-space: nowrap;
}
.time-chip__del {
  border: none;
  background: none;
  color: var(--ink-soft);
  font-size: 12px;
  cursor: pointer;
  padding: 0 2px;
  border-radius: 6px;
}
.time-chip__del:hover {
  color: var(--peach);
}
.time-chip__input {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--bg-window);
  font-family: Consolas, monospace;
  font-size: 12px;
  color: var(--ink);
  padding: 2px 4px;
}
.time-chip__dash {
  color: var(--ink-soft);
}
</style>

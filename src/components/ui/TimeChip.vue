<script setup lang="ts">
/**
 * 时段胶囊（设计方案 §5.1 / §4.3）：mono 展示 start–end + 内嵌时钟图标；
 * hover 显示 ✕ 删除；可编辑变体（editable）下点击进入行内编辑两个 input type=time，
 * 起止可连续修改，✓ / Enter 确认上抛 update，Esc 取消。
 * autoEdit 时挂载即进入编辑态（新增时段场景）。
 * ui/ 红线：零业务依赖，仅受控 range 驱动 + 删除/更新事件。
 */
import { ref, onMounted, nextTick, watch } from "vue";

interface Range {
  start: string;
  end: string;
}

const props = defineProps<{ range: Range; editable?: boolean; autoEdit?: boolean }>();
const emit = defineEmits<{
  (e: "remove"): void;
  (e: "update", range: Range): void;
}>();

const start = ref(props.range.start);
const end = ref(props.range.end);
const editing = ref(false);
const startInput = ref<HTMLInputElement | null>(null);

watch(
  () => props.range,
  (r) => {
    start.value = r.start;
    end.value = r.end;
  },
);

/** 进入编辑态：回填当前值并聚焦起始输入框，便于立即改时段。 */
function enterEdit(): void {
  if (!props.editable) return;
  start.value = props.range.start;
  end.value = props.range.end;
  editing.value = true;
  void nextTick(() => startInput.value?.focus());
}

/** 确认改时段：上抛 update 并退出编辑态（不随单次 change 关闭，可连续改起止）。 */
function confirm(): void {
  editing.value = false;
  emit("update", { start: start.value, end: end.value });
}

/** 取消编辑：还原为外部值并退出。 */
function cancel(): void {
  start.value = props.range.start;
  end.value = props.range.end;
  editing.value = false;
}

function onRemove(): void {
  emit("remove");
}

onMounted(() => {
  if (props.autoEdit) enterEdit();
});
</script>

<template>
  <span
    class="time-chip"
    :class="{ 'is-editing': editing }"
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
      <span
        class="time-chip__text"
        @click="enterEdit"
      >{{ range.start }} – {{ range.end }}</span>
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
        ref="startInput"
        v-model="start"
        class="time-chip__input"
        type="time"
        aria-label="开始时间"
        @keydown.enter="confirm"
        @keydown.esc="cancel"
      />
      <span class="time-chip__dash">–</span>
      <input
        v-model="end"
        class="time-chip__input"
        type="time"
        aria-label="结束时间"
        @keydown.enter="confirm"
        @keydown.esc="cancel"
      />
      <button
        class="time-chip__ok"
        type="button"
        aria-label="完成"
        @click.stop="confirm"
      >
        ✓
      </button>
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
.time-chip__ok {
  border: none;
  background: var(--matcha);
  color: #fff;
  font-size: 11px;
  line-height: 1;
  cursor: pointer;
  padding: 3px 5px;
  border-radius: 6px;
}
.time-chip__ok:hover {
  background: var(--matcha-deep);
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

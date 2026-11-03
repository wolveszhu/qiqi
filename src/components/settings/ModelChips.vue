<script setup lang="ts">
/**
 * 模型标签输入（chips）：回车/逗号添加、空输入退格删尾项、点 ✕ 删除。
 *
 * 受控组件：`modelValue: string[]` + `update:modelValue`；`suggestions` 为平台预设
 * 常见模型（已添加的不再展示）——用户仍可输入任意模型名（预设只是"常见"）。
 */

import { computed, ref } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: string[];
    /** 快速添加建议（预设平台常见模型）。 */
    suggestions?: string[];
    placeholder?: string;
  }>(),
  {
    suggestions: () => [],
    placeholder: "输入模型名，回车 / 逗号添加",
  },
);

const emit = defineEmits<{ (e: "update:modelValue", value: string[]): void }>();

const draft = ref("");

/** 尚未添加的建议项（最多展示 4 个，避免刷屏）。 */
const remainingSuggestions = computed(() =>
  props.suggestions.filter((s) => !props.modelValue.includes(s)).slice(0, 4),
);

/** 添加一个模型（去重、去空白；空值忽略）。 */
function add(raw: string): void {
  const name = raw.trim();
  if (!name) return;
  if (props.modelValue.includes(name)) {
    draft.value = "";
    return;
  }
  emit("update:modelValue", [...props.modelValue, name]);
  draft.value = "";
}

/** 删除一个模型。 */
function remove(name: string): void {
  emit(
    "update:modelValue",
    props.modelValue.filter((m) => m !== name),
  );
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Enter" || e.key === "," || e.key === "，") {
    e.preventDefault();
    add(draft.value);
    return;
  }
  // 空输入 + 退格 → 删掉最后一个标签（与常见标签输入交互一致）
  if (e.key === "Backspace" && draft.value === "" && props.modelValue.length > 0) {
    e.preventDefault();
    remove(props.modelValue[props.modelValue.length - 1] as string);
  }
}
</script>

<template>
  <div class="chips">
    <span
      v-for="m in modelValue"
      :key="m"
      class="chips__item"
    >
      <span class="chips__text">{{ m }}</span>
      <button
        class="chips__del"
        type="button"
        :aria-label="`删除模型 ${m}`"
        @click="remove(m)"
      >
        ✕
      </button>
    </span>

    <input
      class="chips__input"
      :value="draft"
      :placeholder="modelValue.length ? '继续添加…' : placeholder"
      @input="draft = ($event.target as HTMLInputElement).value"
      @keydown="onKeydown"
      @blur="add(draft)"
    />

    <button
      v-for="s in remainingSuggestions"
      :key="s"
      class="chips__suggest"
      type="button"
      @click="add(s)"
    >
      + {{ s }}
    </button>
  </div>
</template>

<style scoped>
.chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: var(--r-sm);
  background: var(--bg-soft);
  transition: border-color var(--dur-fast) var(--ease-out);
}
.chips:focus-within {
  border-color: var(--matcha);
}
.chips__item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px 3px 10px;
  border-radius: 999px;
  background: var(--matcha-deep);
  color: #fff;
  font-size: 12px;
  line-height: 1.5;
}
.chips__text {
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.chips__del {
  border: none;
  background: none;
  color: rgb(255 255 255 / 0.75);
  font-size: 10px;
  cursor: pointer;
  padding: 0 2px;
  border-radius: 50%;
}
.chips__del:hover {
  color: #fff;
  background: rgb(255 255 255 / 0.2);
}
.chips__input {
  flex: 1;
  min-width: 140px;
  border: none;
  background: none;
  outline: none;
  font: 500 13px var(--font-body);
  color: var(--ink);
  padding: 4px 2px;
}
.chips__input::placeholder {
  color: var(--ink-soft);
}
.chips__suggest {
  border: 1px dashed var(--line);
  background: none;
  border-radius: 999px;
  padding: 3px 10px;
  font-size: 12px;
  color: var(--ink-soft);
  cursor: pointer;
  transition:
    color var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out);
}
.chips__suggest:hover {
  color: var(--matcha-deep);
  border-color: var(--matcha);
}
</style>

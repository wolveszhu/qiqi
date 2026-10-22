<script setup lang="ts" generic="T extends string">
/**
 * 分段器（设计方案 §5.1）：单选项容器，受控（modelValue + update:modelValue）。
 * 支持 small 副标（9.5px）。ui/ 红线：零业务依赖，仅泛型选项驱动。
 */
interface SegmentedOption {
  value: T;
  label: string;
  /** 副标（如成本档/次数说明）。 */
  sub?: string;
}

withDefaults(
  defineProps<{
    modelValue: T;
    options: SegmentedOption[];
    small?: boolean;
  }>(),
  { small: false },
);

const emit = defineEmits<{ (e: "update:modelValue", value: T): void }>();

function select(value: T): void {
  emit("update:modelValue", value);
}
</script>

<template>
  <div
    class="seg"
    :class="{ 'seg--small': small }"
    role="radiogroup"
  >
    <button
      v-for="o in options"
      :key="o.value"
      class="seg__item"
      :class="{ 'is-active': o.value === modelValue }"
      role="radio"
      :aria-checked="o.value === modelValue"
      @click="select(o.value)"
    >
      <span class="seg__label">{{ o.label }}</span>
      <span
        v-if="o.sub"
        class="seg__sub"
        >{{ o.sub }}</span
      >
    </button>
  </div>
</template>

<style scoped>
.seg {
  display: inline-flex;
  gap: 4px;
  padding: 4px;
  background: var(--bg-deep);
  border-radius: 13px;
}
.seg__item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1px;
  min-width: 72px;
  padding: 6px 14px;
  border: none;
  border-radius: 10px;
  background: none;
  cursor: pointer;
  color: var(--ink-soft);
  font-family: var(--font-body);
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
}
.seg--small .seg__item {
  min-width: 56px;
  padding: 5px 10px;
}
.seg__item.is-active {
  background: var(--bg-window);
  color: var(--matcha-deep);
  font-weight: 700;
  box-shadow: var(--shadow-card);
}
.seg__label {
  font-size: 13px;
  font-weight: 600;
}
.seg--small .seg__label {
  font-size: 12px;
}
.seg__sub {
  font-size: 9.5px;
  color: var(--ink-soft);
  opacity: 0.85;
}
.seg__item.is-active .seg__sub {
  color: var(--matcha-deep);
}
</style>

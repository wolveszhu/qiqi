<script setup lang="ts">
/**
 * 铃铛勿扰开关（D4.3）：inline SVG 代码绘制（零位图资源）。
 * 正常 = 抹茶描边铃铛；勿扰 = 灰 + 斜杠角标。点击区 ≥32px，按下变形防误触。
 * 仅负责 UI 与 emit（toggle 逻辑在 PetWindow：写库 + 事件同步 + toast）。
 */
defineProps<{ enabled: boolean }>();
const emit = defineEmits<{ (e: "toggle"): void }>();

function onPress(): void {
  emit("toggle");
}
</script>

<template>
  <button
    class="bell"
    :class="{ 'bell--on': enabled }"
    type="button"
    :aria-label="enabled ? '关闭勿扰' : '开启勿扰'"
    :aria-pressed="enabled"
    @click="onPress"
  >
    <svg
      class="bell__svg"
      viewBox="0 0 24 24"
      width="22"
      height="22"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" />
      <path d="M13.7 21a2 2 0 0 1-3.4 0" />
      <line
        v-if="enabled"
        class="bell__slash"
        x1="3"
        y1="3"
        x2="21"
        y2="21"
      />
    </svg>
  </button>
</template>

<style scoped>
.bell {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: var(--bg-window);
  color: var(--matcha-deep);
  cursor: pointer;
  box-shadow: 0 1px 4px var(--shadow-soft);
  transition:
    transform 0.12s ease,
    color 0.15s ease;
}
.bell:hover {
  transform: scale(1.08);
}
.bell:active {
  transform: scale(0.9);
}
.bell--on {
  color: var(--ink-soft);
}
.bell__svg {
  display: block;
}
.bell__slash {
  stroke: var(--peach);
  stroke-width: 2.2;
}
</style>

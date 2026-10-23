<script setup lang="ts">
/**
 * 预览舞台（设计方案 §4.1 / §5.2，红线）：
 * 内嵌**独立** PIXI 实例（factory.createRenderer 直调，不走窗口 A 单例），
 * PIXI Application 由**普通变量**持有（严禁进 reactive/ref）；点击调 playAction('click')；
 * onBeforeUnmount / 切换形象时 dispose + 清空容器，杜绝实例泄漏（冷眼三问③）。
 */
import { onBeforeUnmount, onMounted, ref, watch } from "vue";

import { createRenderer } from "../../renderer/factory";
import type { IPetRenderer } from "../../renderer/IPetRenderer";

const props = defineProps<{ rendererId: string }>();

const stageEl = ref<HTMLElement | null>(null);
let renderer: IPetRenderer | null = null;

function dispose(): void {
  if (renderer) {
    renderer.dispose();
    renderer = null;
  }
  if (stageEl.value) stageEl.value.innerHTML = "";
}

async function build(id: string): Promise<void> {
  dispose();
  if (!stageEl.value) return;
  try {
    renderer = createRenderer(id);
    await renderer.mount(stageEl.value);
  } catch (e) {
    console.warn("[preview] 渲染器初始化失败（已降级 CSS 占位）", e);
  }
}

function onClick(): void {
  renderer?.playAction("click");
}

onMounted((): void => {
  void build(props.rendererId);
});
onBeforeUnmount(dispose);

// 切换形象 → 销毁重建（独立实例，窗口 B 与 A 天然隔离）
watch(
  () => props.rendererId,
  (id): void => {
    void build(id);
  },
);
</script>

<template>
  <div
    ref="stageEl"
    class="preview-stage"
    role="img"
    aria-label="桌宠实时预览"
    @click="onClick"
  >
    <span class="preview-stage__badge">LIVE PREVIEW · 窗口 A</span>
    <span class="preview-stage__hint">点击角色体验果冻物理</span>
  </div>
</template>

<style scoped>
.preview-stage {
  position: relative;
  /* 正方形舞台 + 居中：配合 CodeCatRenderer 基于容器尺寸居中，形象始终正中间完整展示 */
  width: 100%;
  max-width: 264px;
  aspect-ratio: 1 / 1;
  margin: 0 auto;
  border-radius: 18px;
  background: linear-gradient(160deg, #fffdf4, #fdf3dd);
  overflow: hidden;
  cursor: pointer;
}
.preview-stage__badge {
  position: absolute;
  top: 10px;
  left: 12px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 1px;
  color: var(--ink-soft);
  background: rgba(255, 255, 255, 0.6);
  padding: 2px 8px;
  border-radius: 999px;
}
.preview-stage__hint {
  position: absolute;
  bottom: 10px;
  left: 0;
  right: 0;
  text-align: center;
  font-size: 11px;
  color: var(--ink-soft);
}
</style>

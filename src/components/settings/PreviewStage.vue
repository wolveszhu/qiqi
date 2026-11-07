<script setup lang="ts">
/**
 * 预览舞台（设计方案 §4.1 / §5.2，红线）：
 * 内嵌**独立** PIXI 实例（factory.createRenderer 直调，不走窗口 A 单例），
 * PIXI Application 由**普通变量**持有（严禁进 reactive/ref）；点击调 playAction('click')；
 * onBeforeUnmount / 切换形象时 dispose + 清空容器，杜绝实例泄漏（冷眼三问③）。
 */
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import { createRenderer } from "../../renderer/factory";
import type { IPetRenderer } from "../../renderer/IPetRenderer";

const { t } = useI18n();

const props = defineProps<{ rendererId: string }>();

const stageEl = ref<HTMLElement | null>(null);
let renderer: IPetRenderer | null = null;

/**
 * 构建串行链（并发安全）。
 *
 * `mount()` 是异步的（`Application.init` 要等 WebGL 就绪）。若第一次构建还没 append 完，
 * 第二次构建就开始了，两次 `mount()` 都会往同一个容器 append canvas ——
 * 表现就是预览里**同时出现猫和狗两只**。串行化保证"先彻底销毁，再挂新的"。
 */
let buildChain: Promise<void> = Promise.resolve();

function dispose(): void {
  if (renderer) {
    renderer.dispose();
    renderer = null;
  }
  // 兜底清空：`destroy()` 不保证把 canvas 从 DOM 摘掉
  if (stageEl.value) stageEl.value.innerHTML = "";
}

async function doBuild(id: string): Promise<void> {
  dispose();
  const el = stageEl.value;
  if (!el) return;
  const next = createRenderer(id);
  renderer = next;
  try {
    await next.mount(el);
  } catch (e) {
    console.warn("[preview] 渲染器初始化失败（已降级 CSS 占位）", e);
  }
}

/** 入队一次构建（排队执行，永不并发）。 */
function build(id: string): void {
  buildChain = buildChain
    .then(() => doBuild(id))
    .catch((e: unknown) => {
      console.warn("[preview] 渲染器构建异常", e);
    });
}

function onClick(): void {
  renderer?.playAction("click");
}

onMounted((): void => {
  build(props.rendererId);
});

onBeforeUnmount((): void => {
  // 若仍有排队中的构建，上面的 `stageEl.value` 会变 null 使其直接返回
  buildChain = Promise.resolve();
  dispose();
});

// 切换形象 → 销毁重建（独立实例，窗口 B 与 A 天然隔离）
watch(
  () => props.rendererId,
  (id): void => {
    build(id);
  },
);
</script>

<template>
  <div
    ref="stageEl"
    class="preview-stage"
    role="img"
    :aria-label="t('preview.aria')"
    @click="onClick"
  >
    <span class="preview-stage__badge">{{ t("preview.badge") }}</span>
    <span class="preview-stage__hint">{{ t("preview.hint") }}</span>
  </div>
</template>

<style scoped>
.preview-stage {
  position: relative;
  /* 正方形舞台 + 居中：配合渲染器基类基于容器尺寸居中，形象始终正中间完整展示 */
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

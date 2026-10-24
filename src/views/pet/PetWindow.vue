<script setup lang="ts">
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

import SpeechBubble from "../../components/pet/SpeechBubble.vue";
import {
  lookAt,
  playAction,
  setExpression,
  setIdleMode,
  setEdgeSleep,
  usePetRenderer,
} from "../../composables/usePetRenderer";
import { proactiveBubble, useProactive } from "../../composables/useProactive";
import { useAffection } from "../../composables/useAffection";
import { useSystemProbe } from "../../composables/useSystemProbe";
import { settingsStore, applyDndLocal, applySnapshot } from "../../state/settingsStore";
import { settleAffection, sleepDecay } from "../../api/pet";
import { onChatToggle } from "../../api/probe";
import {
  getOuterPosition,
  restoreFromSnap,
  setBubbleMode,
  setSleepMode,
  show,
  snapToEdge,
  startDrag,
} from "../../api/window";
import { getTyped } from "../../api/settings";
import { detectSnapEdge } from "../../utils/edgeSnap";

/** 休眠气泡文案（D4.4：只露耳+眼时，窗口 A 旁边挂一个 💤 小气泡，窗口保持展开态）。 */
const SLEEP_BUBBLE = "💤 z z z…";

const unlisteners: UnlistenFn[] = [];
const isDragging = ref(false);
const sleepSnap = ref(false);

// ---- E2.3① 待机降帧：PIXI maxFPS 单一真源（probeIdle || sleepSnap → 30fps，否则 60fps）----
let probeIdle = false;
function refreshIdleFps(): void {
  // 入睡(C6 idle)或吸附休眠(D4)任一成立即降帧；唤醒交互即时回升 60fps
  setIdleMode(probeIdle || sleepSnap.value);
}

// ---- 主链路接线（E1a：此前三个 composable 未在窗口 A 实例化，MVP 链路断裂） ----
usePetRenderer();
useProactive();
useAffection();
useSystemProbe({
  // 鼠标坐标 → 视线跟随（C2/C6 探针 → 渲染器 lookAt）
  onMove: (x: number, y: number) => lookAt(x, y),
  // 空闲翻转 → 睡/醒动作 + 待机降帧（C6：30min 无操作睡、回归伸懒腰）
  onIdleChange: (idle: boolean) => {
    probeIdle = idle;
    if (idle) playAction("sleep");
    else playAction("stretch");
    refreshIdleFps();
  },
});

// 气泡数据源（窗口 A 单气泡覆盖式）：**只显示主动话术**——
// 对话的流式回复已迁到窗口 B「对话」Tab，A 不再显示（用户决策）。
// 休眠态覆盖为 💤 小气泡（D4.4）：主动引擎已在 sleepSnap 下 skip，故兜底用固定文案。
const bubbleText = computed(() =>
  sleepSnap.value ? SLEEP_BUBBLE : proactiveBubble.text,
);
const bubbleStreaming = computed(() => !sleepSnap.value && proactiveBubble.isStreaming);
const bubbleVisible = computed(() => bubbleText.value.length > 0);

// ---- 气泡出现 → 窗口 A 临时展开（Rust 侧保持中心：形象屏幕位置不跳） ----
// 窗口常态贴合形象，主动话术气泡需要横向空间，故按需放大；消失后收起。
watch(
  bubbleVisible,
  (expanded: boolean) => {
    void setBubbleMode(expanded).catch((e: unknown) =>
      console.warn("[window] 窗口展开态切换失败", e),
    );
  },
  { immediate: true },
);

/**
 * 打开窗口 B 的对话 Tab（双击形象 / Ctrl+Shift+P）。
 * 双窗口独立 webview、store 不共享：A 只负责 show + 发 `window:chat:open`，
 * 切换 Tab 与聚焦由 B 自己完成。
 */
async function openChatInSettings(): Promise<void> {
  // D4.4：休眠期间屏蔽对话入口（双击 / Ctrl+Shift+P 均不打开对话 Tab）
  if (sleepSnap.value) return;
  try {
    await show("settings");
    await emit("window:chat:open", {});
  } catch (e: unknown) {
    console.warn("[pet] 打开对话窗口失败", e);
  }
}

function onAction(name: string): void {
  if (name === "pet") playAction("click");
  else if (name === "drag") playAction("drag");
  else if (name === "sleep") playAction("sleep");
  else if (name === "wake") playAction("stretch");
}

// ---- D4.4 拖拽 → 边缘吸附休眠（仅在开关开启时触发）----
async function onDragEnd(): Promise<void> {
  isDragging.value = false;
  try {
    const { x, y, monitor } = await getOuterPosition();
    const edge = detectSnapEdge({ x, y }, monitor);
    // 进入休眠：开关开启 + 命中边缘（上/左/右，下边不吸附）
    if (edge && settingsStore.snapSleepEnabled) {
      await snapToEdge(edge);
      sleepSnap.value = true;
      settingsStore.sleepSnap = true;
      setEdgeSleep(true); // 只露耳+眼 + 逆时针 90°（D4.4 视觉休眠）
      refreshIdleFps(); // 吸附休眠 → 待机帧率（E2.3①）
      // 后台联动：广播 Rust 休眠态（日记静默）+ 衰减锚点（enter 先结算睡前累计）
      void setSleepMode(true).catch((e: unknown) =>
        console.warn("[pet] 休眠态广播失败", e),
      );
      void sleepDecay("enter").catch((e: unknown) =>
        console.warn("[pet] 休眠衰减结算失败", e),
      );
    } else if (sleepSnap.value) {
      // 拖离边缘 → 唤醒（仅拖动唤醒，单击不唤醒）
      await restoreFromSnap();
      sleepSnap.value = false;
      settingsStore.sleepSnap = false;
      setEdgeSleep(false); // 恢复完整形象
      refreshIdleFps(); // 拖离 → 恢复（仍受 probeIdle 影响）
      // 后台联动：解除休眠态 + 衰减锚点前移（exit 赦免休眠天数，醒来再续）
      void setSleepMode(false).catch((e: unknown) =>
        console.warn("[pet] 休眠态解除失败", e),
      );
      void sleepDecay("exit").catch((e: unknown) =>
        console.warn("[pet] 休眠衰减锚点前移失败", e),
      );
    }
  } catch (e: unknown) {
    console.warn("[pet] 贴边检测失败（降级：不吸附）", e);
  }
}

// ---- 手势分配（窗口贴合形象后，命中区=猫身主体）----
// 单击：抚摸（果冻反馈 + 好感计分）；按住移动超阈值：系统接管拖拽；双击：打开输入框。
/** 拖拽触发阈值（px）：低于此视为点击/双击，避免双击被系统拖拽抢走。 */
const DRAG_THRESHOLD_PX = 5;
let pressX = 0;
let pressY = 0;
let dragStarted = false;

function onPointerMove(e: PointerEvent): void {
  if (dragStarted) return;
  const dx = e.clientX - pressX;
  const dy = e.clientY - pressY;
  if (dx * dx + dy * dy < DRAG_THRESHOLD_PX * DRAG_THRESHOLD_PX) return;
  dragStarted = true;
  isDragging.value = true;
  void startDrag(); // 系统接管拖拽（C2）：移动/跨屏/DPI 均由 OS 处理
}

function onPointerUp(): void {
  window.removeEventListener("pointermove", onPointerMove);
  // 仅真正发生过拖拽才做贴边检测（风险#12：拖拽中首次 pointerup 触发一次）
  if (dragStarted) void onDragEnd();
}

function onPointerDown(e: PointerEvent): void {
  pressX = e.clientX;
  pressY = e.clientY;
  dragStarted = false;
  if (sleepSnap.value) {
    // 休眠期：不抚摸、不互动（D4.4：仅拖动可唤醒）；仍注册拖拽监听以便拖离边缘
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp, { once: true });
    return;
  }
  playAction("click");
  // 抚摸计分（M1 锚点④：点击猫身即抚摸+1，日上限 20；C5 内部结算与上限控制）
  void settleAffection({ kind: "petStroke" }).catch((e: unknown) =>
    console.warn("[pet] 抚摸计分失败", e),
  );
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp, { once: true });
}

/** 双击形象 → 打开窗口 B 的对话 Tab（休眠期屏蔽，见 openChatInSettings 守卫）。 */
function onDoubleClick(): void {
  void openChatInSettings();
}

onMounted(async (): Promise<void> => {
  unlisteners.push(
    await listen<string>("pet:action", (event) => onAction(event.payload)),
  );
  // 窗口 B 改勿扰 → A 同步（双窗口独立 webview，store 不共享，靠事件）
  unlisteners.push(
    await listen<{ enabled: boolean }>("settings:dnd:changed", (event) => {
      if (settingsStore.snapshot) {
        applyDndLocal({
          enabled: event.payload.enabled,
          ranges: settingsStore.snapshot.dnd.ranges,
        });
      }
    }),
  );
  // 窗口 A 首次挂载拉取设置快照（开关状态 snapSleepEnabled 依赖；窗口 B 才写快照）
  if (!settingsStore.snapshot) {
    void getTyped()
      .then(applySnapshot)
      .catch((e: unknown) => {
        console.warn("[pet] 设置快照拉取失败（使用默认 true）", e);
      });
  }
  // Ctrl+Shift+P → 打开窗口 B 的对话 Tab（D1；对话已迁至 B；休眠期由 openChatInSettings 屏蔽）
  unlisteners.push(
    await onChatToggle(() => {
      if (sleepSnap.value) return;
      void openChatInSettings();
    }),
  );
  // 窗口 B 的对话流结束 → A 做表情联动（B 无渲染器句柄，只能靠事件）
  unlisteners.push(
    await listen<{ sessionId: string }>("chat:done", () => {
      setExpression("happy");
    }),
  );
});

onUnmounted((): void => {
  unlisteners.forEach((fn) => fn());
});
</script>

<template>
  <div
    class="pet-root"
    @pointerdown="onPointerDown"
    @dblclick="onDoubleClick"
  >
    <div id="pet-canvas"></div>
    <SpeechBubble
      :text="bubbleText"
      :streaming="bubbleStreaming"
      :visible="bubbleVisible"
    />
  </div>
</template>

<style scoped>
.pet-root {
  position: relative;
  width: 100%;
  height: 100%;
}
#pet-canvas {
  width: 100%;
  height: 100%;
  display: block;
}
.bubble-layer {
  position: absolute;
  top: -8px;
  left: 50%;
  transform: translateX(-50%);
  width: 360px;
  pointer-events: none;
  z-index: 5;
}
</style>

<script setup lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

import SpeechBubble from "../../components/pet/SpeechBubble.vue";
import ChatInput from "../../components/pet/ChatInput.vue";
import {
  lookAt,
  playAction,
  setIdleMode,
  setPetSleeping,
  usePetRenderer,
} from "../../composables/usePetRenderer";
import { proactiveBubble, useProactive } from "../../composables/useProactive";
import { useChatStream } from "../../composables/useChatStream";
import { useAffection } from "../../composables/useAffection";
import { useSystemProbe } from "../../composables/useSystemProbe";
import { settingsStore, applyDndLocal } from "../../state/settingsStore";
import { chatStore } from "../../state/chatStore";
import { settleAffection } from "../../api/pet";
import { onChatToggle } from "../../api/probe";
import {
  focusMainWindow,
  getOuterPosition,
  restoreFromSnap,
  setBubbleMode,
  setClickThroughEnabled,
  snapToEdge,
  startDrag,
} from "../../api/window";
import { detectSnapEdge } from "../../utils/edgeSnap";

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
const stream = useChatStream();
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

// 气泡数据源（窗口 A 单气泡覆盖式，勘误#12）：对话流优先，主动话术次之
const bubbleText = computed(() => chatStore.text || proactiveBubble.text);
const bubbleStreaming = computed(
  () => chatStore.isStreaming || proactiveBubble.isStreaming,
);
const bubbleVisible = computed(() => bubbleText.value.length > 0);

// ---- 气泡/输入框出现 → 窗口 A 临时展开（Rust 侧保持中心：形象屏幕位置不跳） ----
// 窗口常态贴合形象，气泡与输入框需要横向空间，故按需放大；消失后收起。
const windowExpanded = computed(
  () => bubbleVisible.value || settingsStore.chatInputVisible,
);
watch(
  windowExpanded,
  (expanded: boolean) => {
    void setBubbleMode(expanded).catch((e: unknown) =>
      console.warn("[window] 窗口展开态切换失败", e),
    );
  },
  { immediate: true },
);

// ---- 输入框显隐 → 穿透总闸 + 窗口聚焦（气泡与输入框可各自独立出现，故单独 watch） ----
// 输入框期间必须关闭穿透：否则点击落到下层窗口，既打不了字也点不到关闭；
// 同时窗口 set_size 展开会让 webview 失焦，故展开后主动聚焦窗口，保证 textarea 拿到焦点。
watch(
  () => settingsStore.chatInputVisible,
  async (visible: boolean) => {
    try {
      await setClickThroughEnabled(!visible);
      if (visible) await focusMainWindow();
    } catch (e: unknown) {
      console.warn("[window] 穿透总闸切换/聚焦失败", e);
    }
  },
);

function onSend(text: string): void {
  stream.send(text);
}
function onCloseInput(): void {
  settingsStore.chatInputVisible = false;
}

function onAction(name: string): void {
  if (name === "pet") playAction("click");
  else if (name === "drag") playAction("drag");
  else if (name === "sleep") playAction("sleep");
  else if (name === "wake") playAction("stretch");
}

// ---- D4.4 拖拽 → 边缘吸附休眠 ----
async function onDragEnd(): Promise<void> {
  isDragging.value = false;
  try {
    const { x, y, monitor } = await getOuterPosition();
    const edge = detectSnapEdge({ x, y }, monitor);
    if (edge) {
      await snapToEdge(edge);
      sleepSnap.value = true;
      settingsStore.sleepSnap = true;
      setPetSleeping(true); // 角色变暗（D4.4 视觉休眠；窗口级 opacity 在 tauri 2.12.4 不可用）
      refreshIdleFps(); // 吸附休眠 → 待机帧率（E2.3①）
    } else if (sleepSnap.value) {
      await restoreFromSnap();
      sleepSnap.value = false;
      settingsStore.sleepSnap = false;
      setPetSleeping(false); // 唤醒恢复不透明
      refreshIdleFps(); // 拖离 → 恢复（仍受 probeIdle 影响）
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
  playAction("click");
  // 抚摸计分（M1 锚点④：点击猫身即抚摸+1，日上限 20；C5 内部结算与上限控制）
  void settleAffection({ kind: "petStroke" }).catch((e: unknown) =>
    console.warn("[pet] 抚摸计分失败", e),
  );
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp, { once: true });
}

/** 双击形象 → 打开对话输入框（Ctrl+Shift+P 仍保留为 toggle 通道）。 */
function onDoubleClick(): void {
  settingsStore.chatInputVisible = true;
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
  // Ctrl+Shift+P 呼出/隐藏输入框（D1；toggle 运行时 UI 态）
  unlisteners.push(
    await onChatToggle(() => {
      settingsStore.chatInputVisible = !settingsStore.chatInputVisible;
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
    <ChatInput
      class="chat-layer"
      :visible="settingsStore.chatInputVisible"
      @pointerdown.stop
      @send="onSend"
      @close="onCloseInput"
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
.chat-layer {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 15;
  pointer-events: auto;
}
</style>

<script setup lang="ts">
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, onMounted, onUnmounted, ref } from "vue";

import SpeechBubble from "../../components/pet/SpeechBubble.vue";
import ChatInput from "../../components/pet/ChatInput.vue";
import BellToggle from "../../components/pet/BellToggle.vue";
import {
  lookAt,
  playAction,
  setExtraHitRegion,
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
import { saveDnd } from "../../api/settings";
import { settleAffection } from "../../api/pet";
import { onChatToggle } from "../../api/probe";
import {
  getOuterPosition,
  restoreFromSnap,
  snapToEdge,
  startDrag,
} from "../../api/window";
import { detectSnapEdge } from "../../utils/edgeSnap";
import { pushToast } from "../../composables/useToast";

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

function onPointerDown(): void {
  // 系统接管拖拽（C2）；拖拽中首次 pointerup 触发一次吸附检测（风险#12）
  void startDrag();
  playAction("click");
  // 抚摸计分（M1 锚点④：点击猫身即抚摸+1，日上限 20；C5 内部结算与上限控制）
  void settleAffection({ kind: "petStroke" }).catch((e: unknown) =>
    console.warn("[pet] 抚摸计分失败", e),
  );
  if (!isDragging.value) {
    isDragging.value = true;
    window.addEventListener("pointerup", () => void onDragEnd(), { once: true });
  }
}

// ---- D4.3 铃铛勿扰（双向同步：写库 + 事件 + A 本地乐观） ----
async function onBellToggle(): Promise<void> {
  const cur = settingsStore.snapshot?.dnd.enabled ?? false;
  const next = !cur;
  const ranges = settingsStore.snapshot?.dnd.ranges ?? [];
  applyDndLocal({ enabled: next, ranges }); // 乐观
  try {
    await saveDnd({ enabled: next, ranges }, settingsStore.frequency);
    void emit("settings:dnd:changed", { enabled: next });
    pushToast(next ? "已进入勿扰，主动找你聊天已静默" : "已关闭勿扰");
  } catch (e: unknown) {
    applyDndLocal({ enabled: cur, ranges }); // 回滚
    pushToast(e instanceof Error ? e.message : "勿扰切换失败", "error");
  }
}

onMounted(async (): Promise<void> => {
  unlisteners.push(
    await listen<string>("pet:action", (event) => onAction(event.payload)),
  );
  // 窗口 B 改勿扰 → A 铃铛同步（双窗口独立 webview，store 不共享，靠事件）
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
  // 铃铛矩形 union 进 hitRegion（D4.3：穿透状态机照常）
  setExtraHitRegion({ x: 8, y: 8, width: 32, height: 32 });
});

onUnmounted((): void => {
  unlisteners.forEach((fn) => fn());
  setExtraHitRegion(null);
});
</script>

<template>
  <div
    class="pet-root"
    @pointerdown="onPointerDown"
  >
    <div id="pet-canvas"></div>
    <BellToggle
      class="pet-bell"
      :enabled="settingsStore.snapshot?.dnd.enabled ?? false"
      @pointerdown.stop
      @toggle="onBellToggle"
    />
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
  width: 400px;
  height: 400px;
}
#pet-canvas {
  width: 400px;
  height: 400px;
  display: block;
}
.pet-bell {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 10;
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

<script setup lang="ts">
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

import SpeechBubble from "../../components/pet/SpeechBubble.vue";
import {
  lookAt,
  onRendererRebuilt,
  playAction,
  setExpression,
  setIdleMode,
  setEdgeSleep,
  forceRebuildCurrent,
  usePetRenderer,
} from "../../composables/usePetRenderer";
import { proactiveBubble, useProactive } from "../../composables/useProactive";
import { useAffection } from "../../composables/useAffection";
import { useSystemProbe } from "../../composables/useSystemProbe";
import { petStore } from "../../state/petStore";
import { settingsStore, applyDndLocal, applySnapshot } from "../../state/settingsStore";
import { settleAffection } from "../../api/pet";
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
import { detectSnapEdge, resolveSnapDecision, type SnapEdge } from "../../utils/edgeSnap";

const unlisteners: UnlistenFn[] = [];
const isDragging = ref(false);
/** 已贴边吸附（系统级默认行为，与静默开关无关；true = 躺倒露出中）。 */
const sleepSnap = ref(false);
/** 当前贴边侧（吸附中改比例时复用；null = 未吸附）。 */
let snapEdge: SnapEdge | null = null;
/** 首次吸附的强制重建兜底只做一次（进程内）。 */
let snapRendererWarmedUp = false;

// ---- E2.3① 待机降帧：PIXI maxFPS 单一真源（probeIdle || sleepSnap → 30fps，否则 60fps）----
let probeIdle = false;
function refreshIdleFps(): void {
  // 入睡(C6 idle)或贴边吸附(D4)任一成立即降帧；拖离边缘即时回升 60fps
  setIdleMode(probeIdle || sleepSnap.value);
}

/**
 * 切换贴边静默态：**唯一效果是让主动引擎 skip（不主动发消息）**。
 *
 * 用户决策（2026-11-13）：静默开启时对话、抚摸、快捷键、后台日记与好感度衰减
 * 一律照常，故此处只写内存态，不再广播 Rust `sleep_mode` / 结算衰减锚点。
 * 吸附本身是常开的系统默认行为，与静默完全无关。
 */
function setSilent(next: boolean): void {
  settingsStore.snapSilent = next;
}

// ---- 主链路接线（E1a：此前三个 composable 未在窗口 A 实例化，MVP 链路断裂） ----
usePetRenderer();
/**
 * 形象重建完成后重放用户可见状态。
 *
 * 新渲染器是"出厂态"：躺倒姿态与降帧标志都会丢，若不重放，
 * 换个形象会顺手把已吸附的桌宠姿态**复位**。
 */
onRendererRebuilt((): void => {
  if (sleepSnap.value && snapEdge) {
    setEdgeSleep(true, snapEdge, settingsStore.snapShowRatio);
  }
  refreshIdleFps();
});
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
// 贴边静默时主动引擎 skip，故无气泡；单纯吸附（开关关闭）照常显示主动话术。
const bubbleText = computed(() => proactiveBubble.text);
const bubbleStreaming = computed(() => proactiveBubble.isStreaming);
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
  // D4.5：贴边静默只停主动话术，用户主动的对话入口（双击 / 快捷键）始终可用
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

// ---- D4.5 拖拽 → 边缘吸附（系统默认常开）；静默由开关单独控制 ----
// 风险#12 深修：系统拖拽（start_dragging）接管鼠标事件后，WebView 会在**拖拽开始瞬间**
// 收到一个"捕获丢失型" pointerup（此时按键未松开），且松手后**不再有** pointerup
// （Windows 模态拖拽循环吞掉）。因此不能靠 pointerup 判定拖拽结束：
// 改为轮询窗口位置，连续稳定 DRAG_STABLE_POLLS 次（~600ms 无位移）视为拖拽结束。
const DRAG_POLL_INTERVAL_MS = 120;
const DRAG_STABLE_POLLS = 5;
let dragTracking = false; // 轮询循环进行中
let dragPollAbort = false; // 真松开 → 取消轮询（立即判定）
let evaluatingDrag = false; // 判定重入守卫（轮询/pointerup 双路径竞态）

async function onDragEnd(): Promise<void> {
  if (evaluatingDrag) return;
  evaluatingDrag = true;
  isDragging.value = false;
  try {
    const { x, y, width, height, monitor } = await getOuterPosition();
    const edge = detectSnapEdge({ x, y, width, height }, monitor);
    // 吸附是系统级默认行为：命中左右边缘即贴边躺倒，与静默开关无关（用户决策 2026-11-13）。
    const decision = resolveSnapDecision(edge, settingsStore.snapSleepEnabled);
    if (decision.snap && edge) {
      console.warn("[snap] onDragEnd", {
        edge,
        x,
        y,
        width,
        height,
        ratio: settingsStore.snapShowRatio,
      });
      await snapToEdge(edge);
      snapEdge = edge;
      sleepSnap.value = true;
      settingsStore.sleepSnap = true;
      setEdgeSleep(true, edge, settingsStore.snapShowRatio); // 整体躺下：左=顺时针 / 右=逆时针
      refreshIdleFps(); // 吸附 → 待机帧率（E2.3①）
      setSilent(decision.silent); // 仅开关开启时停止主动话术

      // 窗口 `set_position` 之后 WebView 布局 / PIXI `resizeTo`（ResizeObserver
      // → ticker）异步生效，晚 1–2 帧。首次吸附紧接 await 就算姿态，用的是窗口
      // 移动前的几何 → 形象被推出边缘、命中区越界；切换形象之所以自愈，是因为
      // 重新 `mount()` 发生在布局稳定之后。这里在两帧后按稳定尺寸重放一次，
      // 每次都由 syncViewport() 现取 app.screen 并立即补报命中区；守卫防止
      // 期间已拖离边缘时把姿态又放回去。
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          if (!sleepSnap.value || snapEdge !== edge) return;
          setEdgeSleep(true, edge, settingsStore.snapShowRatio);
        });
      });

      // 兜底：首次吸附按当前形象重建一次，走「切换形象」那条已验证正常的路径。
      // 与上面的重放幂等，afterRebuild 会在新渲染器上再重放一次姿态。
      if (!snapRendererWarmedUp) {
        snapRendererWarmedUp = true;
        void forceRebuildCurrent().catch((e: unknown) =>
          console.warn("[snap] 首次吸附重建失败", e),
        );
      }
    } else if (sleepSnap.value) {
      // 拖离边缘 → 复原姿态（仅拖动可离开边缘，单击不处理）
      await restoreFromSnap();
      snapEdge = null;
      sleepSnap.value = false;
      settingsStore.sleepSnap = false;
      setEdgeSleep(false); // 旋转归零，恢复原姿态
      refreshIdleFps(); // 拖离 → 恢复（仍受 probeIdle 影响）
      setSilent(false);
    }
  } catch (e: unknown) {
    console.warn("[pet] 贴边检测失败（降级：不吸附）", e);
  } finally {
    evaluatingDrag = false;
  }
}

/** 拖拽结束追踪：轮询窗口位置直至稳定（仅拖拽期间，~120ms/次 IPC）。 */
async function trackDragEnd(): Promise<void> {
  if (dragTracking) return;
  dragTracking = true;
  dragPollAbort = false;
  try {
    let lastX = Number.NaN;
    let lastY = Number.NaN;
    let stable = 0;
    while (stable < DRAG_STABLE_POLLS && !dragPollAbort) {
      await new Promise((resolve) => setTimeout(resolve, DRAG_POLL_INTERVAL_MS));
      if (dragPollAbort) break;
      try {
        const { x, y } = await getOuterPosition();
        if (x === lastX && y === lastY) stable++;
        else {
          stable = 0;
          lastX = x;
          lastY = y;
        }
      } catch (e: unknown) {
        console.warn("[pet] 拖拽追踪取位失败（放弃本轮判定）", e);
        return;
      }
    }
    if (!dragPollAbort) await onDragEnd();
  } finally {
    dragTracking = false;
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
  void trackDragEnd(); // 轮询判定拖拽结束（系统拖拽吞 pointerup，见风险#12 深修注释）
}

function onPointerUp(e: PointerEvent): void {
  window.removeEventListener("pointermove", onPointerMove);
  if (!dragStarted) return;
  if (e.buttons !== 0) {
    // 捕获丢失型 pointerup（按键未松开，拖拽刚开始瞬间触发）——
    // 不做贴边判定，交给 trackDragEnd 轮询；否则会在窗口还在屏幕中间时提前判定。
    return;
  }
  // 真松开：取消轮询、立即判定（轮询路径由 evaluatingDrag 守卫防重入）
  dragPollAbort = true;
  void onDragEnd();
}

/**
 * 窗口 A 禁用 WebView2 原生右键菜单（"将图像另存为" / "复制图像" / "检查"）。
 *
 * Tauri v2 无关闭该菜单的配置项：Windows 侧由 WebView2 宿主层默认开启
 * （wry 的 `default_context_menus = true`），而 DOM `contextmenu` 事件的
 * `preventDefault()` 可阻止其弹出（微软 WebView2 文档的自定义菜单做法）。
 * 只在本窗口（PetWindow 挂载的 webview）注册，窗口 B 输入框的复制/粘贴
 * 右键菜单不受影响；也不改 devtools 开关，开发调试仍可用。
 */
function onContextMenu(e: MouseEvent): void {
  e.preventDefault();
}

function onPointerDown(e: PointerEvent): void {
  // 只响应主键：右键/中键不参与抚摸计分与系统拖拽。
  // 右键菜单已被 onContextMenu 屏蔽，若继续放行会误 +1 好感度、甚至启动拖拽。
  // 左键单击 / 双击 / 拖拽路径完全不变。
  if (e.button !== 0) return;
  pressX = e.clientX;
  pressY = e.clientY;
  dragStarted = false;
  // 贴边静默不影响抚摸与拖拽（D4.5：只停主动话术）
  playAction("click");
  // 抚摸计分（M1 锚点④：点击猫身即抚摸+1，日上限 20；C5 内部结算与上限控制）
  void settleAffection({ kind: "petStroke" }).catch((e: unknown) =>
    console.warn("[pet] 抚摸计分失败", e),
  );
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp, { once: true });
}

/** 双击形象 → 打开窗口 B 的对话 Tab（贴边静默期同样可用）。 */
function onDoubleClick(): void {
  void openChatInSettings();
}

onMounted(async (): Promise<void> => {
  // 旧版本静默会把 Rust `sleep_mode` 置位以暂停日记/衰减；新语义下后台任务照常。
  // 启动时清一次历史遗留标志（该态为内存态，重启本就为 false，此处覆盖热更新场景）。
  void setSleepMode(false).catch((e: unknown) =>
    console.warn("[pet] 后台状态复位失败", e),
  );
  // 兜底：capture 阶段拦截整窗右键，覆盖 .pet-root 之外的节点与运行时插入的
  // PIXI canvas（模板只挂在根元素，冒泡链一旦被改动就会漏）。
  // 仅 preventDefault、不 stopPropagation：不阻断其他事件，未来要在此窗口做
  // 自定义右键菜单时仍可在子元素自行处理。
  document.addEventListener("contextmenu", onContextMenu, true);
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
  // 窗口 B 改贴边静默开关 → A 同步；已吸附时即时启停主动话术（吸附本身不受影响）
  unlisteners.push(
    await listen<{ enabled: boolean }>("settings:snap_sleep:changed", (event) => {
      settingsStore.snapSleepEnabled = event.payload.enabled;
      if (settingsStore.snapshot) {
        settingsStore.snapshot.snapSleepEnabled = event.payload.enabled;
      }
      setSilent(sleepSnap.value && event.payload.enabled);
    }),
  );
  // 窗口 B 改吸附露出比例 → A 同步（吸附中改比例即时生效）
  unlisteners.push(
    await listen<{ ratio: number }>("settings:snap_ratio:changed", (event) => {
      settingsStore.snapShowRatio = event.payload.ratio;
      if (settingsStore.snapshot) {
        settingsStore.snapshot.snapShowRatio = event.payload.ratio;
      }
      // 吸附中改比例即时生效（沿用当前贴边侧）
      if (sleepSnap.value && snapEdge) setEdgeSleep(true, snapEdge, event.payload.ratio);
    }),
  );
  // 窗口 B 切换形象 → 只写 store；重建由 usePetRenderer 的 watch 统一执行
  // （单一重建路径：启动读取与 B 窗口推送都走同一条，不会出现两处各自重建）
  unlisteners.push(
    await listen<string>("pet:renderer:changed", (event) => {
      if (event.payload) petStore.activeRenderer = event.payload;
    }),
  );
  // 窗口 A 首次挂载拉取设置快照（贴边静默开关初值依赖；窗口 B 才写快照）
  if (!settingsStore.snapshot) {
    void getTyped()
      .then(applySnapshot)
      .catch((e: unknown) => {
        console.warn("[pet] 设置快照拉取失败（使用默认 true）", e);
      });
  }
  // Ctrl+Shift+P → 打开窗口 B 的对话 Tab（D1；贴边静默期同样可用）
  unlisteners.push(
    await onChatToggle(() => {
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
  document.removeEventListener("contextmenu", onContextMenu, true);
  unlisteners.forEach((fn) => fn());
});
</script>

<template>
  <div
    class="pet-root"
    @pointerdown="onPointerDown"
    @dblclick="onDoubleClick"
    @contextmenu="onContextMenu"
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

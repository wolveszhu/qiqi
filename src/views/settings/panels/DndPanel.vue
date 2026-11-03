<script setup lang="ts">
import { onMounted, onUnmounted, reactive, ref, computed, watch } from "vue";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import AppCard from "../../../components/ui/AppCard.vue";
import TimeChip from "../../../components/ui/TimeChip.vue";
import KbdChip from "../../../components/ui/KbdChip.vue";
import SegmentedControl from "../../../components/ui/SegmentedControl.vue";
import { pushToast } from "../../../composables/useToast";
import {
  applySnapshot,
  applyDndLocal,
  settingsStore,
  setFrequency,
} from "../../../state/settingsStore";
import { getTyped, saveDnd } from "../../../api/settings";
import type { DndConfig, ProactiveFrequency } from "../../../api/types";

const FREQ_OPTIONS = [
  { value: "quiet", label: "安静", sub: "3-5 次/天" },
  { value: "normal", label: "正常", sub: "5-8 次/天" },
  { value: "sticky", label: "粘人", sub: "10+ 次/天" },
];

const dnd = reactive<DndConfig>({ enabled: false, ranges: [] });
const frequency = ref<ProactiveFrequency>("normal");
/** 最近一次新增的时段下标，用于让新胶囊自动进入编辑态。 */
const autoEditIndex = ref(-1);
let saveTimer: ReturnType<typeof setTimeout> | null = null;
let initialized = false;

function toMin(s: string): number | null {
  const m = /^(\d{1,2}):(\d{2})$/.exec(s.trim());
  return m ? Number(m[1]) * 60 + Number(m[2]) : null;
}

/** 分钟数转 HH:MM（用于挑选默认空档）。 */
function fmt(min: number): string {
  const h = Math.floor(min / 60) % 24;
  const m = min % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

/**
 * 在已有勿扰时段之外挑第一段空闲 ≥1h 的窗口作为新时段默认值，
 * 避免「添加即重叠」触发保存拦截（验收：新增不再必为 9:00–18:00）。
 */
function findFreeSlot(ranges: ReadonlyArray<{ start: string; end: string }>): {
  start: string;
  end: string;
} {
  const segs = ranges
    .map((r) => ({ s: toMin(r.start), e: toMin(r.end) }))
    .filter((x): x is { s: number; e: number } => x.s != null && x.e != null && x.s < x.e)
    .sort((a, b) => a.s - b.s);
  let cursor = 0;
  for (const seg of segs) {
    if (seg.s - cursor >= 60) return { start: fmt(cursor), end: fmt(cursor + 60) };
    cursor = Math.max(cursor, seg.e);
  }
  if (24 * 60 - cursor >= 60) return { start: fmt(cursor), end: fmt(cursor + 60) };
  return { start: "09:00", end: "10:00" };
}

/** 存在 start>=end 的非法段 → 阻止保存。 */
const invalidRanges = computed(() =>
  dnd.ranges.some((r) => {
    const s = toMin(r.start);
    const e = toMin(r.end);
    return s == null || e == null || s >= e;
  }),
);

/** 相邻时段交叉 → 阻止保存（验收：重叠被阻止）。 */
const overlapWarn = computed(() => {
  const segs = dnd.ranges
    .map((r) => ({ s: toMin(r.start), e: toMin(r.end) }))
    .filter((x): x is { s: number; e: number } => x.s != null && x.e != null && x.s < x.e)
    .sort((a, b) => a.s - b.s);
  for (let i = 1; i < segs.length; i++) {
    if (segs[i].s < segs[i - 1].e) return true;
  }
  return false;
});

async function ensureInit(): Promise<void> {
  if (settingsStore.snapshot == null) {
    try {
      applySnapshot(await getTyped());
    } catch (e) {
      console.warn("[dnd] 配置拉取失败", e);
    }
  }
  if (initialized) return;
  const snap = settingsStore.snapshot;
  if (snap) {
    dnd.enabled = snap.dnd.enabled;
    dnd.ranges = snap.dnd.ranges.map((r) => ({ ...r }));
    frequency.value = (snap.frequency as ProactiveFrequency) || "normal";
    initialized = true;
  }
}

function persist(): void {
  // 乐观刷新（C6 引擎即时读取 frequency / 勿扰判定）
  applyDndLocal({ enabled: dnd.enabled, ranges: dnd.ranges.map((r) => ({ ...r })) });
  setFrequency(frequency.value);
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout((): void => {
    void doSave();
  }, 300);
}

function rollback(): void {
  const snap = settingsStore.snapshot?.dnd;
  if (snap) {
    dnd.enabled = snap.enabled;
    dnd.ranges = snap.ranges.map((r) => ({ ...r }));
  }
  if (settingsStore.snapshot?.frequency) {
    frequency.value = settingsStore.snapshot.frequency;
  }
  applyDndLocal({ enabled: dnd.enabled, ranges: dnd.ranges.map((r) => ({ ...r })) });
  setFrequency(frequency.value);
}

async function doSave(): Promise<void> {
  if (overlapWarn.value) {
    pushToast("勿扰时段存在重叠，请调整", "error");
    rollback();
    return;
  }
  if (invalidRanges.value) {
    pushToast("时段起止需满足 start < end", "error");
    rollback();
    return;
  }
  try {
    await saveDnd(
      { enabled: dnd.enabled, ranges: dnd.ranges.map((r) => ({ ...r })) },
      frequency.value,
    );
    // 同步窗口 A 铃铛（D4.3 双向）
    void emit("settings:dnd:changed", { enabled: dnd.enabled });
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "保存失败", "error");
    rollback();
  }
}

function addRange(): void {
  const slot = findFreeSlot(dnd.ranges);
  dnd.ranges.push(slot);
  autoEditIndex.value = dnd.ranges.length - 1;
  persist();
}
function removeRange(i: number): void {
  dnd.ranges.splice(i, 1);
  if (autoEditIndex.value === i) autoEditIndex.value = -1;
  else if (autoEditIndex.value > i) autoEditIndex.value -= 1;
  persist();
}
function updateRange(i: number, r: { start: string; end: string }): void {
  dnd.ranges[i] = { ...r };
  persist();
}

watch([() => dnd.enabled, () => dnd.ranges, frequency], persist, { deep: true });

let dndUnlisten: UnlistenFn | null = null;

onMounted(async () => {
  await ensureInit();
  // 窗口 A 铃铛改勿扰 → B 同步（双窗口独立 webview，store 不共享，靠事件）
  dndUnlisten = await listen<{ enabled: boolean }>("settings:dnd:changed", (e) => {
    dnd.enabled = e.payload.enabled;
    applyDndLocal({ enabled: dnd.enabled, ranges: dnd.ranges.map((r) => ({ ...r })) });
  });
});

onUnmounted(() => {
  dndUnlisten?.();
});
</script>

<template>
  <div class="dnd-grid">
    <!-- 勿扰时段 -->
    <AppCard title="勿扰时段">
      <AppToggle
        v-model="dnd.enabled"
        label="启用勿扰（静默主动互动）"
      />
      <div class="dnd-grid__ranges">
        <TimeChip
          v-for="(r, i) in dnd.ranges"
          :key="i"
          :range="r"
          :editable="true"
          :auto-edit="i === autoEditIndex"
          @remove="removeRange(i)"
          @update="(nr) => updateRange(i, nr)"
        />
        <button
          class="dnd-grid__add"
          type="button"
          @click="addRange"
        >
          ＋ 添加时段
        </button>
      </div>
      <HintBox
        v-if="invalidRanges"
        tone="warn"
      >
        每段时段需满足 开始时间 &lt; 结束时间。
      </HintBox>
      <HintBox
        v-else-if="overlapWarn"
        tone="warn"
      >
        存在重叠的勿扰时段，请调整后再保存。
      </HintBox>
    </AppCard>

    <!-- 主动互动频率 -->
    <AppCard title="主动互动频率">
      <SegmentedControl
        v-model="frequency"
        :options="FREQ_OPTIONS"
      />
      <HintBox tone="info">
        🐙 时间窗口内随机触发，拒绝准点打卡——安静档偶尔想你，粘人档挂你身上。
      </HintBox>
    </AppCard>

    <!-- 全局快捷键（只读展示；自启开关后端 IPC 待补） -->
    <AppCard title="全局快捷键">
      <div class="dnd-grid__rows">
        <div class="set-row">
          <div>
            <div class="set-row__title">唤起输入框</div>
            <div class="set-row__sub">在桌宠窗口唤起对话输入</div>
          </div>
          <KbdChip keys="Ctrl+Shift+P" />
        </div>
        <div class="set-row">
          <div>
            <div class="set-row__title">隐藏 / 显示桌宠</div>
            <div class="set-row__sub">全局切换窗口 A 显隐</div>
          </div>
          <KbdChip keys="Ctrl+Shift+H" />
        </div>
        <div class="set-row">
          <div>
            <div class="set-row__title">开机自启</div>
            <div class="set-row__sub">经由托盘菜单设置（前端开关待接入）</div>
          </div>
          <AppToggle
            :model-value="false"
            :disabled="true"
          />
        </div>
      </div>
    </AppCard>

    <!-- 情境感知（探针配置持久化后端命令缺失，占位） -->
    <AppCard title="情境感知">
      <div class="dnd-grid__rows">
        <div class="set-row">
          <div>
            <div class="set-row__title">工作 / 游戏让路</div>
            <div class="set-row__sub">主动概率 −80%</div>
          </div>
          <AppToggle
            :model-value="true"
            :disabled="true"
          />
        </div>
        <div class="set-row">
          <div>
            <div class="set-row__title">空闲 30 分钟睡觉</div>
            <div class="set-row__sub">回归时伸懒腰</div>
          </div>
          <AppToggle
            :model-value="true"
            :disabled="true"
          />
        </div>
        <div class="set-row">
          <div>
            <div class="set-row__title">连续 3 天未互动触发傲娇</div>
            <div class="set-row__sub">依赖探针配置持久化</div>
          </div>
          <AppToggle
            :model-value="true"
            :disabled="true"
          />
        </div>
      </div>
      <HintBox tone="info">
        情境感知开关的后端持久化命令将在后续阶段补齐（本面板不造假状态）。
      </HintBox>
    </AppCard>
  </div>
</template>

<style scoped>
.dnd-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}
.dnd-grid__ranges {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.dnd-grid__add {
  padding: 4px 12px;
  border: 1px dashed var(--line);
  border-radius: 11px;
  background: none;
  color: var(--matcha-deep);
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
}
.dnd-grid__add:hover {
  border-color: var(--matcha);
}
.dnd-grid__rows {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.set-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.set-row__title {
  font-size: 13.5px;
  color: var(--ink);
  font-weight: 600;
}
.set-row__sub {
  font-size: 12px;
  color: var(--ink-soft);
}
@media (max-width: 1080px) {
  .dnd-grid {
    grid-template-columns: 1fr;
  }
}
</style>

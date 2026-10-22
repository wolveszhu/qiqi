<script setup lang="ts">
/**
 * 记忆面板（设计方案 §4.5，D3 实装）：左日记列（倒序 + 重新生成二次确认），右事实标签卡
 * （按 category 分组、conf 降序、行内编辑 + 删除二次确认）。编辑/删除走 memory_* 命令，
 * 后端同步重建 vec_memory 向量（C7）。乐观更新 + 失败回滚。
 */
import { computed, onMounted, ref } from "vue";

import DiaryCard from "../../../components/memory/DiaryCard.vue";
import FactChip from "../../../components/memory/FactChip.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import {
  deleteFact,
  listDiaries,
  listFacts,
  regenerateDiary,
  updateFact,
} from "../../../api/memory";
import { pushToast } from "../../../composables/useToast";
import type { DiaryDto, FactDto } from "../../../api/types";

const diaries = ref<DiaryDto[]>([]);
const facts = ref<FactDto[]>([]);
const loading = ref(false);

// category：0=喜好 / 1=日程 / 2=档案（后端 facts 表 TINYINT 约定）
const GROUP_ORDER = [0, 1, 2];
const GROUP_TITLE: Record<number, string> = {
  0: "喜好",
  1: "日程",
  2: "档案",
};

const sortedDiaries = computed(() =>
  [...diaries.value].sort((a, b) => b.diaryDate.localeCompare(a.diaryDate)),
);

const groupedFacts = computed<Record<number, FactDto[]>>(() => {
  const m: Record<number, FactDto[]> = {};
  for (const f of facts.value) {
    (m[f.category] ??= []).push(f);
  }
  for (const k of Object.keys(m)) {
    m[Number(k)].sort((a, b) => b.confidence - a.confidence);
  }
  return m;
});

const groupKeys = computed<number[]>(() =>
  Object.keys(groupedFacts.value)
    .map(Number)
    .sort((a, b) => GROUP_ORDER.indexOf(a) - GROUP_ORDER.indexOf(b)),
);

async function loadAll(): Promise<void> {
  loading.value = true;
  try {
    const [d, f] = await Promise.all([listDiaries(), listFacts()]);
    diaries.value = d;
    facts.value = f;
  } catch (e) {
    console.warn("[memory] 读取失败", e);
    pushToast("记忆读取失败", "error");
  } finally {
    loading.value = false;
  }
}

async function onEditFact(id: number, content: string): Promise<void> {
  const target = facts.value.find((x) => x.id === id);
  if (!target) return;
  const before = target.content;
  target.content = content; // 乐观
  try {
    await updateFact(id, content);
    pushToast("已更新记忆");
  } catch (e) {
    target.content = before; // 回滚
    pushToast(e instanceof Error ? e.message : "更新失败", "error");
  }
}

async function onDeleteFact(id: number): Promise<void> {
  const idx = facts.value.findIndex((x) => x.id === id);
  const removed = idx >= 0 ? facts.value[idx] : null;
  if (idx >= 0) facts.value.splice(idx, 1); // 乐观
  try {
    await deleteFact(id);
    pushToast("已删除记忆");
  } catch (e) {
    if (removed && idx >= 0) facts.value.splice(idx, 0, removed); // 回滚
    pushToast(e instanceof Error ? e.message : "删除失败", "error");
  }
}

async function onRegenerate(date: string): Promise<void> {
  try {
    await regenerateDiary(date);
    await loadAll();
    pushToast("日记已重新生成");
  } catch (e) {
    pushToast(e instanceof Error ? e.message : "重新生成失败", "error");
  }
}

onMounted(loadAll);
</script>

<template>
  <AppCard title="记忆">
    <div class="mem-grid">
      <!-- 左：日记列 -->
      <section class="mem-col">
        <div class="mem-col__head">
          <h3 class="mem-col__title">猫猫日记</h3>
          <span class="mem-col__count">{{ diaries.length }} 篇</span>
        </div>
        <div
          v-if="loading"
          class="mem-loading"
        >
          正在读取日记…
        </div>
        <template v-else>
          <DiaryCard
            v-for="d in sortedDiaries"
            :key="d.id"
            :diary="d"
            @regenerate="onRegenerate"
          />
          <HintBox
            v-if="!diaries.length"
            tone="info"
          >
            她还没写过日记——今晚见。
          </HintBox>
        </template>
      </section>

      <!-- 右：事实标签卡 -->
      <section class="mem-col">
        <div class="mem-col__head">
          <h3 class="mem-col__title">事实标签</h3>
          <span class="mem-col__count">{{ facts.length }} 条</span>
        </div>
        <div
          v-if="loading"
          class="mem-loading"
        >
          正在读取事实…
        </div>
        <template v-else>
          <div
            v-for="cat in groupKeys"
            :key="cat"
            class="mem-group"
          >
            <div class="mem-group__title">
              <span class="mem-group__dot"></span>
              {{ GROUP_TITLE[cat] ?? cat }}
            </div>
            <div class="mem-group__row">
              <FactChip
                v-for="f in groupedFacts[cat]"
                :key="f.id"
                :fact="f"
                @edit="onEditFact"
                @delete="onDeleteFact"
              />
            </div>
          </div>
          <HintBox
            v-if="!facts.length"
            tone="info"
          >
            还没有记住的事——多聊聊天，她会慢慢记下来的。
          </HintBox>
        </template>
      </section>
    </div>

    <HintBox>
      ✏️ 记忆可纠正：编辑 / 删除事实后，向量记忆会同步重建；纠正权始终在你手里。
    </HintBox>
  </AppCard>
</template>

<style scoped>
.mem-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}
.mem-col {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.mem-col__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}
.mem-col__title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
  color: var(--ink);
}
.mem-col__count {
  font-size: 11px;
  color: var(--ink-soft);
  font-family: Consolas, monospace;
}
.mem-loading {
  font-size: 13px;
  color: var(--ink-soft);
}
.mem-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.mem-group__title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  font-weight: 700;
  color: var(--ink);
}
.mem-group__dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--peach);
}
.mem-group__row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
@media (max-width: 1080px) {
  .mem-grid {
    grid-template-columns: 1fr;
  }
}
</style>

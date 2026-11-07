<script setup lang="ts">
/**
 * 记忆面板（设计方案 §4.5，D3 实装）：
 * 日记列（倒序 + 日期锚点条滚动定位 + mood 徽标 + 重新生成二次确认 + loading 防重复）
 * + 事实标签卡（category 分组、conf 降序、行内编辑 / 删除二次确认 / 新增正向入口）。
 * 编辑/删除/新增走 memory_* 命令（C7/D3.1），后端同步重建 vec_memory 向量；乐观更新 + 失败回滚。
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import DiaryCard from "../../../components/memory/DiaryCard.vue";
import FactChip from "../../../components/memory/FactChip.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import HintBox from "../../../components/ui/HintBox.vue";
import {
  addFact,
  deleteFact,
  listDiaries,
  listFacts,
  regenerateDiary,
  updateFact,
} from "../../../api/memory";
import { pushToast } from "../../../composables/useToast";
import type { DiaryDto, FactDto } from "../../../api/types";

const { t } = useI18n();

const diaries = ref<DiaryDto[]>([]);
const facts = ref<FactDto[]>([]);
const loading = ref(false);
const regeneratingDate = ref<string | null>(null);
const adding = ref(false);
const addContent = ref("");
const addCategory = ref(0);

// category：0=喜好 / 1=日程 / 2=档案（后端 facts 表 TINYINT 约定）
const GROUP_ORDER = [0, 1, 2];
const GROUP_KEYS: Record<number, string> = {
  0: "catPref",
  1: "catSchedule",
  2: "catProfile",
};
/** 分类标题（随语言切换）。 */
function groupTitle(cat: number): string {
  return t(`memory.${GROUP_KEYS[cat] ?? "catProfile"}`);
}
const today = new Date().toISOString().slice(0, 10);

const sortedDiaries = computed(() =>
  [...diaries.value].sort((a, b) => b.diaryDate.localeCompare(a.diaryDate)),
);

const dateAnchors = computed(() => {
  const s = new Set(diaries.value.map((d) => d.diaryDate));
  return [...s].sort((a, b) => b.localeCompare(a));
});

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
    pushToast(t("memory.readFailed"), "error");
  } finally {
    loading.value = false;
  }
}

async function onEditFact(id: number, content: string): Promise<void> {
  const fact = facts.value.find((x) => x.id === id);
  if (!fact) return;
  const before = fact.content;
  fact.content = content; // 乐观
  try {
    await updateFact(id, content);
    pushToast(t("memory.updated"));
  } catch (e) {
    fact.content = before; // 回滚
    pushToast(e instanceof Error ? e.message : t("memory.updateFailed"), "error");
  }
}

async function onDeleteFact(id: number): Promise<void> {
  const idx = facts.value.findIndex((x) => x.id === id);
  const removed = idx >= 0 ? facts.value[idx] : null;
  if (idx >= 0) facts.value.splice(idx, 1); // 乐观
  try {
    await deleteFact(id);
    pushToast(t("memory.deleted"));
  } catch (e) {
    if (removed && idx >= 0) facts.value.splice(idx, 0, removed); // 回滚
    pushToast(e instanceof Error ? e.message : t("memory.deleteFailed"), "error");
  }
}

async function onRegenerate(date: string): Promise<void> {
  regeneratingDate.value = date; // 防重复点击
  try {
    await regenerateDiary(date);
    await loadAll();
    pushToast(t("memory.diaryRegenerated"));
  } catch (e) {
    pushToast(e instanceof Error ? e.message : t("memory.regenFailed"), "error");
  } finally {
    regeneratingDate.value = null;
  }
}

async function onAddFact(): Promise<void> {
  const content = addContent.value.trim();
  if (!content) {
    pushToast(t("memory.emptyContent"), "error");
    return;
  }
  if (content.length > 200) {
    pushToast(t("memory.contentTooLong"), "error");
    return;
  }
  adding.value = true;
  try {
    await addFact(content, addCategory.value);
    addContent.value = "";
    await loadAll();
    pushToast(t("memory.added"));
  } catch (e) {
    pushToast(e instanceof Error ? e.message : t("memory.addFailed"), "error");
  } finally {
    adding.value = false;
  }
}

function scrollToDate(date: string): void {
  document
    .getElementById(`diary-${date}`)
    ?.scrollIntoView({ behavior: "smooth", block: "center" });
}

onMounted(loadAll);
</script>

<template>
  <AppCard :title="t('memory.title')">
    <div class="mem-grid">
      <!-- 左：日记列 -->
      <section class="mem-col">
        <div class="mem-col__head">
          <h3 class="mem-col__title">{{ t("memory.diary") }}</h3>
          <span class="mem-col__count">{{
            t("memory.diaryCount", { n: diaries.length })
          }}</span>
        </div>
        <div
          v-if="dateAnchors.length"
          class="date-anchors"
        >
          <button
            v-for="d in dateAnchors"
            :key="d"
            class="date-anchor"
            :class="{ 'is-today': d === today }"
            type="button"
            @click="scrollToDate(d)"
          >
            {{ d.slice(5) }}
          </button>
        </div>
        <div
          v-if="loading"
          class="mem-loading"
        >
          {{ t("memory.loadingDiary") }}
        </div>
        <template v-else>
          <div
            v-for="d in sortedDiaries"
            :id="`diary-${d.diaryDate}`"
            :key="d.id"
            class="diary-anchor-wrap"
          >
            <DiaryCard
              :diary="d"
              :loading="regeneratingDate === d.diaryDate"
              @regenerate="onRegenerate"
            />
          </div>
          <HintBox
            v-if="!diaries.length"
            tone="info"
          >
            {{ t("memory.noDiary") }}
          </HintBox>
        </template>
      </section>

      <!-- 右：事实标签卡 -->
      <section class="mem-col">
        <div class="mem-col__head">
          <h3 class="mem-col__title">{{ t("memory.facts") }}</h3>
          <span class="mem-col__count">{{
            t("memory.factCount", { n: facts.length })
          }}</span>
        </div>
        <div
          v-if="loading"
          class="mem-loading"
        >
          {{ t("memory.loadingFact") }}
        </div>
        <template v-else>
          <div
            v-for="cat in groupKeys"
            :key="cat"
            class="mem-group"
          >
            <div class="mem-group__title">
              <span class="mem-group__dot"></span>
              {{ groupTitle(cat) }}
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

          <!-- 新增记忆入口（D3.1 正向入口） -->
          <div class="mem-add">
            <button
              v-if="!adding"
              class="mem-add__btn"
              type="button"
              @click="adding = true"
            >
              {{ t("memory.addFact") }}
            </button>
            <div
              v-else
              class="mem-add__form"
            >
              <input
                v-model="addContent"
                class="mem-add__input"
                :placeholder="t('memory.addPlaceholder')"
                maxlength="200"
                @keyup.enter="onAddFact"
              />
              <select
                v-model.number="addCategory"
                class="mem-add__select"
              >
                <option :value="0">{{ t("memory.catPref") }}</option>
                <option :value="1">{{ t("memory.catSchedule") }}</option>
                <option :value="2">{{ t("memory.catProfile") }}</option>
              </select>
              <button
                class="mem-add__ok"
                type="button"
                :disabled="!addContent.trim()"
                @click="onAddFact"
              >
                {{ t("common.save") }}
              </button>
              <button
                class="mem-add__cancel"
                type="button"
                @click="adding = false"
              >
                {{ t("common.cancel") }}
              </button>
            </div>
          </div>

          <HintBox
            v-if="!facts.length"
            tone="info"
          >
            {{ t("memory.noFact") }}
          </HintBox>
        </template>
      </section>
    </div>

    <HintBox>
      {{ t("memory.correctHint") }}
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
.date-anchors {
  display: flex;
  gap: 6px;
  overflow-x: auto;
  padding-bottom: 4px;
}
.date-anchor {
  flex: none;
  padding: 3px 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--bg-window);
  font-size: 11px;
  font-family: Consolas, monospace;
  color: var(--ink-soft);
  cursor: pointer;
}
.date-anchor.is-today {
  border-color: var(--matcha);
  color: var(--matcha-deep);
  font-weight: 700;
}
.diary-anchor-wrap {
  scroll-margin: 12px;
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
.mem-add {
  margin-top: 4px;
}
.mem-add__btn {
  width: 100%;
  padding: 8px;
  border: 1px dashed var(--line);
  border-radius: 12px;
  background: none;
  color: var(--matcha-deep);
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
}
.mem-add__btn:hover {
  border-color: var(--matcha);
}
.mem-add__form {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.mem-add__input {
  flex: 1;
  min-width: 120px;
  padding: 6px 10px;
  border: 1px solid var(--matcha);
  border-radius: 8px;
  font-size: 12px;
  color: var(--ink);
  background: var(--bg-window);
}
.mem-add__select {
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  font-size: 12px;
  color: var(--ink);
  background: var(--bg-window);
}
.mem-add__ok {
  padding: 6px 12px;
  border: none;
  border-radius: 8px;
  background: var(--matcha);
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}
.mem-add__ok:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.mem-add__cancel {
  padding: 6px 10px;
  border: none;
  border-radius: 8px;
  background: var(--bg-deep);
  color: var(--ink-soft);
  font-size: 12px;
  cursor: pointer;
}
@media (max-width: 1080px) {
  .mem-grid {
    grid-template-columns: 1fr;
  }
}
</style>

<script setup lang="ts">
/**
 * 划词翻译浮窗根视图（第三窗口）：监听选区事件 → 显示小图标 → 点击后取词翻译。
 *
 * 纪律：所有 await IPC 都 try/catch 并按 AppError.kind 用 t() 给本地化反馈，
 * 绝不把 Rust 中文 message 直出给用户；凭据与原文不进日志。
 */

import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { normalizeError } from "../../api/client";
import {
  captureSelection,
  copyTranslateResult,
  executeTranslate,
  hideTranslatePopup,
  retryTranslate,
  showTranslatePopup,
} from "../../api/translate";
import type { TranslateResult, TranslateSelectionPayload } from "../../api/types";
import AppToast from "../../components/ui/AppToast.vue";
import { pushToast } from "../../composables/useToast";
import SelectionToolbar from "./SelectionToolbar.vue";
import TranslationPopup from "./TranslationPopup.vue";

const { t, te } = useI18n();

const toolbarVisible = ref(false);
const selectionKey = ref("");
const anchor = ref({ x: 0, y: 0 });
const result = ref<TranslateResult | null>(null);
const busy = ref(false);
const unlisteners: UnlistenFn[] = [];
let hideTimer: number | null = null;

/** 按错误 kind 本地化提示，避免展示后端中文 message。 */
function localizedError(e: unknown): string {
  const err = normalizeError(e);
  const key = `translate.error.${err.kind}`;
  return te(key) ? t(key) : t("translate.error.unknown");
}

/** 点击小图标：这才发生取词与翻译（手势阶段只读鼠标状态）。 */
async function onCapture(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const captured = await captureSelection(selectionKey.value);
    result.value = await executeTranslate(captured.text);
    await showTranslatePopup(anchor.value.x, anchor.value.y);
    toolbarVisible.value = false;
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  } finally {
    busy.value = false;
  }
}

/** 一键复制译文。 */
async function onCopy(): Promise<void> {
  if (!result.value) return;
  try {
    await copyTranslateResult(result.value.translatedText);
    pushToast(t("translate.copied"), "ok");
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

/** 重新翻译（同一原文，in-flight 期间禁用按钮）。 */
async function onRetry(): Promise<void> {
  if (!result.value || busy.value) return;
  busy.value = true;
  try {
    result.value = await retryTranslate(result.value.resultId);
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  } finally {
    busy.value = false;
  }
}

/** 收起：隐藏窗口但保留进程内窗口实例。 */
async function onClose(): Promise<void> {
  result.value = null;
  toolbarVisible.value = false;
  selectionKey.value = "";
  try {
    await hideTranslatePopup();
  } catch (e: unknown) {
    pushToast(localizedError(e), "error");
  }
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") void onClose();
}

onMounted(async (): Promise<void> => {
  unlisteners.push(
    await listen<TranslateSelectionPayload>("translate:selection:show", (event) => {
      const payload = event.payload;
      if (selectionKey.value === payload.selectionKey) return;
      selectionKey.value = payload.selectionKey;
      anchor.value = { x: payload.cursorX, y: payload.cursorY };
      result.value = null;
      toolbarVisible.value = true;
      // 透明窗口先显示在选区旁，小图标位于窗口左上角；取词时 Rust 会恢复原前台窗口。
      void showTranslatePopup(payload.cursorX, payload.cursorY).catch(() => {});
      if (hideTimer !== null) window.clearTimeout(hideTimer);
      hideTimer = window.setTimeout(() => {
        toolbarVisible.value = false;
      }, 4000);
    }),
  );
  window.addEventListener("keydown", onKeydown);
});

onUnmounted((): void => {
  unlisteners.forEach((fn) => fn());
  window.removeEventListener("keydown", onKeydown);
  if (hideTimer !== null) window.clearTimeout(hideTimer);
});
</script>

<template>
  <div
    class="translator-window"
    @click.self="onClose"
  >
    <SelectionToolbar
      v-if="toolbarVisible && !result"
      :loading="busy"
      @capture="onCapture"
    />
    <TranslationPopup
      v-if="result"
      :result="result"
      :busy="busy"
      @copy="onCopy"
      @retry="onRetry"
      @close="onClose"
    />
    <AppToast />
  </div>
</template>

<style scoped>
.translator-window {
  width: 100vw;
  height: 100vh;
  background: transparent;
  overflow: hidden;
}
</style>

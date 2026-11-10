<script setup lang="ts">
/**
 * 划词翻译浮窗根视图（第三窗口）：选区旁胶囊工具条 → 点击动作后取词 → 译文卡片。
 *
 * 纪律：所有 await IPC 都 try/catch 并按 AppError.kind 用 t() 给本地化反馈，
 * 绝不把 Rust 中文 message 直出给用户；凭据与原文不进日志。
 * 窗口定位：用 Rust 下发的选区包围盒（物理像素），工具条与译文卡片都贴在选中词右下角；
 * 点击窗口之外由全局钩子判定并广播 `translate:selection:hide`。
 */

import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { normalizeError } from "../../api/client";
import {
  captureSelection,
  copyTranslateResult,
  executeTranslate,
  getTranslateSettings,
  hideTranslatePopup,
  listBuiltinTranslateProviders,
  retryTranslate,
  showTranslatePopup,
} from "../../api/translate";
import type {
  SelectionRect,
  TranslateResult,
  TranslateSelectionPayload,
} from "../../api/types";
import AppToast from "../../components/ui/AppToast.vue";
import { pushToast } from "../../composables/useToast";
import SelectionToolbar from "./SelectionToolbar.vue";
import TranslationPopup from "./TranslationPopup.vue";

/** 工具条 / 译文卡片相对选中词右下角的间距（物理像素，Rust 按物理坐标定位）。 */
const ANCHOR_GAP = 6;

/** 选中词右下角锚点；双击时包围盒为 0×0，即落在点击点右下角。 */
function anchorOf(rect: SelectionRect): { x: number; y: number } {
  return {
    x: rect.x + rect.width + ANCHOR_GAP,
    y: rect.y + rect.height + ANCHOR_GAP,
  };
}

const { t, te } = useI18n();

const toolbarVisible = ref(false);
const selectionKey = ref("");
const selectionRect = ref<SelectionRect>({ x: 0, y: 0, width: 0, height: 0 });
const result = ref<TranslateResult | null>(null);
const busy = ref(false);
const unlisteners: UnlistenFn[] = [];

/** 按错误 kind 本地化提示，避免展示后端中文 message。 */
function localizedError(e: unknown): string {
  const err = normalizeError(e);
  const key = `translate.error.${err.kind}`;
  return te(key) ? t(key) : t("translate.error.unknown");
}

/**
 * 配置类错误的进一步诊断：读脱敏配置 + 内置服务商状态，区分「总开关关闭」、
 * 「两家都停用/凭据缺失」、「两家都达月度上限」，让提示可行动。
 * 取词失败已归为 probe，不会走到这里；查询失败仍回落到通用文案。
 */
async function configHint(): Promise<string> {
  try {
    const [settings, builtins] = await Promise.all([
      getTranslateSettings(),
      listBuiltinTranslateProviders(),
    ]);
    if (!settings.enabled) return t("translate.error.configDisabled");
    const usable = builtins.filter((p) => p.enabled && p.hasCredential);
    if (usable.length === 0) return t("translate.error.builtinUnavailable");
    if (usable.every((p) => p.used >= p.monthlyGranted)) {
      return t("translate.error.builtinExhausted");
    }
    if (!settings.hasBuiltinCredential && !settings.hasCustomKey) {
      return t("translate.error.configNoCredential");
    }
    return t("translate.error.config");
  } catch {
    return t("translate.error.config");
  }
}

/** 统一的错误反馈出口：按 kind 本地化，config 额外给出可行动提示。 */
async function reportError(e: unknown): Promise<void> {
  if (normalizeError(e).kind === "config") {
    pushToast(await configHint(), "error");
    return;
  }
  pushToast(localizedError(e), "error");
}

/** 只重置视图状态（Rust 侧已隐藏窗口时不再重复 hide）。 */
function resetState(): void {
  toolbarVisible.value = false;
  result.value = null;
  selectionKey.value = "";
}

/** 点击「翻译」：这才发生取词与翻译（手势阶段只读鼠标状态）。 */
async function onTranslate(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const captured = await captureSelection(selectionKey.value);
    result.value = await executeTranslate(captured.text);
    const anchor = anchorOf(selectionRect.value);
    await showTranslatePopup(anchor.x, anchor.y, false);
    toolbarVisible.value = false;
  } catch (e: unknown) {
    await reportError(e);
  } finally {
    busy.value = false;
  }
}

/** 点击「复制」：只把当前选中的原文写入系统剪贴板，不翻译、也不收起窗口等其他操作。 */
async function onCopySelection(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const captured = await captureSelection(selectionKey.value);
    await copyTranslateResult(captured.text);
    pushToast(t("translate.toolbar.selectionCopied"), "ok");
  } catch (e: unknown) {
    await reportError(e);
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
    await reportError(e);
  }
}

/** 重新翻译（同一原文，in-flight 期间禁用按钮）。 */
async function onRetry(): Promise<void> {
  if (!result.value || busy.value) return;
  busy.value = true;
  try {
    result.value = await retryTranslate(result.value.resultId);
  } catch (e: unknown) {
    await reportError(e);
  } finally {
    busy.value = false;
  }
}

/** 收起：隐藏窗口但保留进程内窗口实例。 */
async function onClose(): Promise<void> {
  resetState();
  try {
    await hideTranslatePopup();
  } catch (e: unknown) {
    await reportError(e);
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
      selectionRect.value = { ...payload.rect };
      result.value = null;
      toolbarVisible.value = true;
      // 紧贴选中词右下角（双击选中词时包围盒为 0×0，即点击点右下角）。
      const anchor = anchorOf(payload.rect);
      void showTranslatePopup(anchor.x, anchor.y, true).catch(() => {});
    }),
    // 点击窗口之外：Rust 已隐藏窗口并清掉矩形，这里只同步视图状态。
    await listen("translate:selection:hide", () => {
      resetState();
    }),
  );
  window.addEventListener("keydown", onKeydown);
});

onUnmounted((): void => {
  unlisteners.forEach((fn) => fn());
  window.removeEventListener("keydown", onKeydown);
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
      @translate="onTranslate"
      @copy="onCopySelection"
      @close="onClose"
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

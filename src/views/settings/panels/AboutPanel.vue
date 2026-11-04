<script setup lang="ts">
/**
 * 关于面板（窗口 B「关于」Tab）：当前版本 + 检查更新 + 更新说明 + 下载入口。
 *
 * 更新检测为轻量自建：仅"提醒 + 跳转下载"，由用户手动运行安装包（规避代码签名）。
 * 状态来自 `useUpdate` 单例（后台周期检查也会写入同一份），本面板每次切回时主动再查一次。
 */

import { onActivated, onMounted } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";

import AppButton from "../../../components/ui/AppButton.vue";
import { pushToast } from "../../../composables/useToast";
import { useUpdate } from "../../../composables/useUpdate";

const { updateInfo, checking, lastError, checkNow, dismiss } = useUpdate();

onMounted((): void => {
  void checkNow();
});

// 切回该 Tab（KeepAlive 激活）时刷新一次状态
onActivated((): void => {
  void checkNow();
});

async function download(): Promise<void> {
  const url = updateInfo.value?.downloadUrl;
  if (!url) return;
  try {
    await openUrl(url);
  } catch (e: unknown) {
    pushToast(e instanceof Error ? e.message : "打开下载链接失败", "error");
  }
}

async function copyLink(): Promise<void> {
  const url = updateInfo.value?.downloadUrl;
  if (!url) return;
  try {
    await navigator.clipboard.writeText(url);
    pushToast("下载链接已复制", "ok");
  } catch {
    pushToast("复制失败，请手动复制链接", "error");
  }
}

/** 检查状态文案。 */
function statusText(): string {
  if (checking.value) return "检查中…";
  if (lastError.value) return `检查失败：${lastError.value}`;
  if (!updateInfo.value) return "尚未检查";
  return updateInfo.value.available
    ? `发现新版本 v${updateInfo.value.latest}`
    : `已是最新版本（v${updateInfo.value.current}）`;
}
</script>

<template>
  <div class="about">
    <header class="about__head">
      <h2 class="about__title">七七</h2>
      <span class="about__ver">v{{ updateInfo?.current ?? "…" }}</span>
    </header>

    <section class="about__check">
      <AppButton
        :loading="checking"
        @click="checkNow"
      >
        检查更新
      </AppButton>
      <span class="about__status">{{ statusText() }}</span>
    </section>

    <section
      v-if="updateInfo?.available"
      class="about__card"
    >
      <div class="about__card-head">
        <span class="about__badge">新版本</span>
        <span class="about__latest">v{{ updateInfo.latest }}</span>
      </div>
      <p
        v-if="updateInfo.notes"
        class="about__notes"
      >
        {{ updateInfo.notes }}
      </p>
      <div class="about__actions">
        <AppButton
          variant="danger"
          :disabled="!updateInfo.downloadUrl"
          @click="download"
        >
          下载更新
        </AppButton>
        <AppButton
          variant="ghost"
          :disabled="!updateInfo.downloadUrl"
          @click="copyLink"
        >
          复制链接
        </AppButton>
        <AppButton
          variant="ghost"
          @click="dismiss"
        >
          稍后
        </AppButton>
      </div>
    </section>

    <p class="about__hint">
      更新包托管于 GitHub
      Releases，下载后手动运行安装即可（当前为轻量自建更新，暂不支持静默安装）。
    </p>
  </div>
</template>

<style scoped>
.about {
  display: flex;
  flex-direction: column;
  gap: 18px;
  max-width: 520px;
}
.about__head {
  display: flex;
  align-items: baseline;
  gap: 10px;
}
.about__title {
  margin: 0;
  font-family: var(--font-disp);
  font-size: 26px;
  font-weight: 400;
  color: var(--ink);
}
.about__ver {
  font-size: 13px;
  color: var(--ink-soft);
  background: var(--bg-deep);
  padding: 2px 10px;
  border-radius: 999px;
}
.about__check {
  display: flex;
  align-items: center;
  gap: 12px;
}
.about__status {
  font-size: 13px;
  color: var(--ink-soft);
}
.about__card {
  background: var(--bg-soft);
  border: 1px solid var(--line);
  border-radius: var(--r-md);
  padding: 16px;
  box-shadow: var(--shadow-card);
}
.about__card-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}
.about__badge {
  font-size: 12px;
  font-weight: 600;
  color: var(--bg-window);
  background: var(--peach);
  padding: 2px 10px;
  border-radius: 999px;
}
.about__latest {
  font-size: 16px;
  font-weight: 700;
  color: var(--matcha-deep);
}
.about__notes {
  margin: 0 0 14px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--ink);
  white-space: pre-wrap;
  word-break: break-word;
}
.about__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
.about__hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--ink-soft);
}
</style>

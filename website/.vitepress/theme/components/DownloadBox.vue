<script setup lang="ts">
import { ref, onMounted } from "vue";

// 只读联动：从构建期拷贝的 /latest.json 读取版本与下载链接，绝不回写。
const version = ref("0.1.0");
const notes = ref("");
const downloadUrl = ref("https://github.com/wolveszhu/qiqi/releases/latest");
const loading = ref(true);

onMounted(async () => {
  try {
    const res = await fetch("/latest.json");
    if (res.ok) {
      const data = await res.json();
      version.value = data.version || version.value;
      notes.value = (data.notes || "").replace(/\\n/g, " ");
      downloadUrl.value = data.downloadUrl || downloadUrl.value;
    }
  } catch (e) {
    console.error("[DownloadBox] 读取 latest.json 失败，使用默认值", e);
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div class="dl jelly-card">
    <div class="dl__badge">{{ loading ? "读取版本中…" : `当前版本 v${version}` }}</div>
    <p
      v-if="notes"
      class="dl__notes"
    >
      {{ notes }}
    </p>
    <a
      class="jelly-btn jelly-btn--primary"
      :href="downloadUrl"
    >
      <svg
        class="ico"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
        <polyline points="7 10 12 15 17 10" />
        <line
          x1="12"
          y1="15"
          x2="12"
          y2="3"
        />
      </svg>
      前往 GitHub Releases 下载
    </a>
    <p class="dl__req">系统要求：Windows 10 及以上（x64） · WebView2 运行时</p>
  </div>
</template>

<style scoped>
.dl {
  max-width: 560px;
  margin: 24px 0;
  padding: 28px;
  text-align: center;
}
.dl__badge {
  display: inline-block;
  font-weight: 700;
  color: var(--qiqi-matcha-deep);
  background: var(--qiqi-matcha-soft);
  border-radius: 999px;
  padding: 6px 16px;
  margin-bottom: 14px;
}
.dl__notes {
  color: var(--qiqi-ink-soft);
  font-size: 14px;
  line-height: 1.6;
  margin: 0 0 18px;
}
.dl__req {
  margin-top: 16px;
  font-size: 13px;
  color: var(--qiqi-ink-soft);
}
</style>

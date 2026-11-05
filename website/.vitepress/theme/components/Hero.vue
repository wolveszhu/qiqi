<script setup lang="ts">
defineProps<{
  title?: string;
  subtitle?: string;
  primaryText?: string;
  primaryLink?: string;
  secondaryText?: string;
  secondaryLink?: string;
  emoji?: string;
}>();
</script>

<template>
  <section class="hero jelly-noise">
    <div class="hero__inner">
      <div class="hero__mascot jelly-float">{{ emoji || "🐾" }}</div>
      <h1 class="hero__title">{{ title }}</h1>
      <p class="hero__subtitle">{{ subtitle }}</p>
      <div class="hero__actions">
        <a
          v-if="primaryLink"
          class="jelly-btn jelly-btn--primary"
          :href="primaryLink"
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
          {{ primaryText }}
        </a>
        <a
          v-if="secondaryLink"
          class="jelly-btn jelly-btn--ghost"
          :href="secondaryLink"
        >
          {{ secondaryText }}
        </a>
      </div>
    </div>
  </section>
</template>

<style scoped>
.hero {
  position: relative;
  overflow: hidden;
  padding: 72px 24px 56px;
  background:
    radial-gradient(1200px 400px at 50% -10%, rgba(124, 166, 91, 0.18), transparent 60%),
    linear-gradient(180deg, #fbf5ea 0%, #efe7d8 100%);
  border-radius: var(--qiqi-r-lg);
  text-align: center;
  margin: 24px auto 0;
  max-width: 1100px;
}
.hero__inner {
  position: relative;
  z-index: 1;
  max-width: 760px;
  margin: 0 auto;
}
.hero__mascot {
  font-size: 76px;
  line-height: 1;
  margin-bottom: 12px;
  filter: drop-shadow(0 10px 18px rgba(93, 84, 60, 0.18));
}
.hero__title {
  font-family: var(--vp-font-family-title);
  font-weight: 400;
  font-size: 46px;
  color: var(--qiqi-ink);
  margin: 0 0 14px;
  letter-spacing: 1px;
}
.hero__subtitle {
  font-size: 18px;
  color: var(--qiqi-ink-soft);
  margin: 0 auto 28px;
  max-width: 560px;
  line-height: 1.7;
}
.hero__actions {
  display: flex;
  gap: 14px;
  justify-content: center;
  flex-wrap: wrap;
}
@media (max-width: 640px) {
  .hero__title {
    font-size: 34px;
  }
}
</style>

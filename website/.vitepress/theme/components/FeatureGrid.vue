<script setup lang="ts">
export interface FeatureItem {
  title: string;
  desc: string;
  emoji?: string;
  image?: string;
}

defineProps<{ features?: FeatureItem[]; columns?: number }>();
</script>

<template>
  <div
    class="fgrid"
    :style="columns ? { '--cols': columns } : {}"
  >
    <article
      v-for="(f, i) in features"
      :key="i"
      class="fcard jelly-pop"
    >
      <div
        v-if="f.image"
        class="fcard__media"
      >
        <img
          :src="f.image"
          :alt="f.title"
          loading="lazy"
        />
      </div>
      <div
        v-else-if="f.emoji"
        class="fcard__emoji"
      >
        {{ f.emoji }}
      </div>
      <h3 class="fcard__title">{{ f.title }}</h3>
      <p class="fcard__desc">{{ f.desc }}</p>
    </article>
  </div>
</template>

<style scoped>
.fgrid {
  display: grid;
  grid-template-columns: repeat(var(--cols, 3), 1fr);
  gap: 18px;
  margin: 28px 0;
}
.fcard {
  background: var(--qiqi-bg-window);
  border: 1.5px solid var(--qiqi-line);
  border-radius: var(--qiqi-r-lg);
  box-shadow: var(--qiqi-shadow-card);
  padding: 22px;
  transition:
    transform 0.2s ease,
    box-shadow 0.2s ease;
}
.fcard:hover {
  transform: translateY(-6px);
  box-shadow: var(--qiqi-shadow-window);
}
.fcard__media img {
  width: 100%;
  height: 168px;
  object-fit: cover;
  border-radius: var(--qiqi-r-md);
  margin-bottom: 14px;
  background: var(--qiqi-matcha-soft);
}
.fcard__emoji {
  font-size: 34px;
  margin-bottom: 12px;
  width: 64px;
  height: 64px;
  display: grid;
  place-items: center;
  background: var(--qiqi-matcha-soft);
  border-radius: 18px;
}
.fcard__title {
  font-family: var(--vp-font-family-title);
  font-weight: 400;
  font-size: 20px;
  margin: 0 0 8px;
  color: var(--qiqi-ink);
}
.fcard__desc {
  font-size: 14px;
  color: var(--qiqi-ink-soft);
  line-height: 1.65;
  margin: 0;
}
@media (max-width: 900px) {
  .fgrid {
    grid-template-columns: repeat(2, 1fr);
  }
}
@media (max-width: 560px) {
  .fgrid {
    grid-template-columns: 1fr;
  }
}
</style>

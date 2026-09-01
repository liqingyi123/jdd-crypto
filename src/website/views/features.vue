<script setup lang="ts">
import { FEATURE_ITEMS } from "../data/features";
import TrailDemo from "../components/trail-demo.vue";
</script>

<template>
  <div class="page-section">
    <div class="container">
      <p class="section-kicker">Features</p>
      <h1 class="section-title display-font">功能介绍</h1>
      <p class="section-lead">围绕加解密日常场景打磨的桌面能力，从角标到插件均可按需开关。</p>

      <div class="feature-grid">
        <article v-for="item in FEATURE_ITEMS" :key="item.id" class="feature-card">
          <h2>{{ item.title }}</h2>
          <p>{{ item.summary }}</p>
        </article>
      </div>

      <section class="trail-block">
        <div class="trail-copy">
          <p class="section-kicker">Live demo</p>
          <h2 class="section-title display-font">鼠标拖尾特效</h2>
          <p class="section-lead">
            与桌面端同一套特效引擎，已在全站跟随鼠标。可在此切换特效，并为绚丽流星 / 连线点阵 / 心动回忆调整颜色。
          </p>
        </div>
        <TrailDemo />
      </section>
    </div>
  </div>
</template>

<style scoped>
.feature-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
  margin: 40px 0 72px;
}

.feature-card {
  padding: 22px 20px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--bg-elevated);
  box-shadow: var(--shadow);
}

.feature-card h2 {
  margin: 0 0 8px;
  font-size: 1.1rem;
}

.feature-card p {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.95rem;
}

.trail-block {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.trail-copy {
  max-width: 40rem;
}
</style>

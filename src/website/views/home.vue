<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { RouterLink } from "vue-router";

const base = import.meta.env.BASE_URL;
const heroRef = ref<HTMLElement | null>(null);

let targetX = 0;
let targetY = 0;
let currentX = 0;
let currentY = 0;
let rafId = 0;
let reducedMotion = false;

function applyParallax() {
  const el = heroRef.value;
  if (!el) {
    return;
  }
  el.style.setProperty("--parallax-x", currentX.toFixed(4));
  el.style.setProperty("--parallax-y", currentY.toFixed(4));
}

function tick() {
  currentX += (targetX - currentX) * 0.1;
  currentY += (targetY - currentY) * 0.1;
  applyParallax();
  if (Math.abs(targetX - currentX) > 0.001 || Math.abs(targetY - currentY) > 0.001) {
    rafId = requestAnimationFrame(tick);
    return;
  }
  rafId = 0;
}

function startTick() {
  if (!rafId) {
    rafId = requestAnimationFrame(tick);
  }
}

function onPointerMove(event: PointerEvent) {
  const el = heroRef.value;
  if (!el) {
    return;
  }
  const rect = el.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) {
    return;
  }
  targetX = Math.max(-1, Math.min(1, ((event.clientX - rect.left) / rect.width - 0.5) * 2));
  targetY = Math.max(-1, Math.min(1, ((event.clientY - rect.top) / rect.height - 0.5) * 2));
  startTick();
}

function onPointerLeave() {
  targetX = 0;
  targetY = 0;
  startTick();
}

onMounted(() => {
  reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const el = heroRef.value;
  if (!el || reducedMotion) {
    return;
  }
  el.addEventListener("pointermove", onPointerMove, { passive: true });
  el.addEventListener("pointerleave", onPointerLeave);
});

onUnmounted(() => {
  const el = heroRef.value;
  if (el) {
    el.removeEventListener("pointermove", onPointerMove);
    el.removeEventListener("pointerleave", onPointerLeave);
  }
  if (rafId) {
    cancelAnimationFrame(rafId);
  }
});
</script>

<template>
  <section ref="heroRef" class="hero">
    <div class="hero-grid" aria-hidden="true" />
    <div class="container hero-inner">
      <img class="hero-logo parallax" :src="`${base}app-icon.png`" width="96" height="96" alt="多多解密" />
      <p class="hero-brand display-font parallax">多多解密</p>
      <h1 class="hero-title display-font parallax">桌面加解密，少打扰多效率</h1>
      <p class="hero-lead parallax">
        奖多多内部专用的跨平台桌面工具：角标常驻、剪贴板询问与结果气泡、鼠标跟随选文、文本对比，以及可玩的鼠标拖尾特效。
      </p>
      <div class="hero-actions">
        <RouterLink class="btn btn-primary" to="/features">了解功能</RouterLink>
        <RouterLink class="btn btn-ghost" to="/changelog">更新日志</RouterLink>
      </div>
    </div>
  </section>
</template>

<style scoped>
.hero {
  --parallax-x: 0;
  --parallax-y: 0;
  position: relative;
  isolation: isolate;
  min-height: calc(100vh - 64px);
  display: flex;
  align-items: center;
  background: var(--hero-glow);
  overflow: hidden;
}

.hero-grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(var(--grid) 1px, transparent 1px),
    linear-gradient(90deg, var(--grid) 1px, transparent 1px);
  background-size: 48px 48px;
  mask-image: radial-gradient(ellipse 70% 60% at 50% 40%, #000 20%, transparent 75%);
  pointer-events: none;
  z-index: -1;
}

.hero-inner {
  padding: 80px 0 100px;
  max-width: 720px;
}

.parallax {
  will-change: transform;
}

.hero-logo {
  display: block;
  width: 96px;
  height: 96px;
  margin-bottom: 20px;
  border-radius: 22px;
  object-fit: contain;
  box-shadow: var(--shadow);
  /* 第 1 行：向左 */
  transform: translate3d(calc(var(--parallax-x) * -28px), calc(var(--parallax-y) * 14px), 0);
}

.hero-brand {
  margin: 0 0 20px;
  font-size: clamp(2.8rem, 8vw, 5.5rem);
  font-weight: 800;
  line-height: 0.95;
  color: var(--text);
  /* 第 2 行：向右 */
  transform: translate3d(calc(var(--parallax-x) * 28px), calc(var(--parallax-y) * 6px), 0);
}

.hero-title {
  margin: 0 0 16px;
  font-size: clamp(1.35rem, 2.8vw, 1.85rem);
  font-weight: 700;
  color: var(--text);
  /* 第 3 行：向左 */
  transform: translate3d(calc(var(--parallax-x) * -28px), calc(var(--parallax-y) * 11px), 0);
}

.hero-lead {
  margin: 0 0 28px;
  max-width: 36rem;
  color: var(--text-muted);
  font-size: 1.08rem;
  /* 第 4 行：向右 */
  transform: translate3d(calc(var(--parallax-x) * 28px), calc(var(--parallax-y) * 4px), 0);
}

.hero-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

@media (prefers-reduced-motion: reduce) {
  .parallax {
    transform: none !important;
    will-change: auto;
  }
}
</style>

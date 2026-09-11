<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, shallowRef } from "vue";

const MIN_SIZE = 100;
const MAX_SIZE = 300;
const DEFAULT_SIZE = 200;
const INNER_FEATHER_PX = 10;
const OUTER_FEATHER_PX = 800;
const MASK_ALPHA = 0.78;
const INNER_START_ALPHA = MASK_ALPHA * 0.5;
const OUTER_START_ALPHA = MASK_ALPHA * 0.92;

const size = shallowRef(DEFAULT_SIZE);
const active = shallowRef(false);
const useCssFullscreen = shallowRef(false);
const stageRef = ref<HTMLElement | null>(null);
const overlayRef = ref<HTMLElement | null>(null);
const holeX = shallowRef("50%");
const holeY = shallowRef("50%");

const maskStyle = computed(() => ({
  "--hole-x": holeX.value,
  "--hole-y": holeY.value,
  "--radius": `${size.value}px`,
  "--inner-feather": `${INNER_FEATHER_PX}px`,
  "--outer-feather": `${OUTER_FEATHER_PX}px`,
  "--mask-alpha": String(MASK_ALPHA),
  "--inner-start": String(INNER_START_ALPHA),
  "--outer-start": String(OUTER_START_ALPHA),
}));

let exiting = false;

function updateHoleFromEvent(event: PointerEvent, root: HTMLElement) {
  const rect = root.getBoundingClientRect();
  holeX.value = `${event.clientX - rect.left}px`;
  holeY.value = `${event.clientY - rect.top}px`;
}

function onStagePointerMove(event: PointerEvent) {
  if (active.value) {
    return;
  }
  const stage = stageRef.value;
  if (!stage) {
    return;
  }
  updateHoleFromEvent(event, stage);
}

async function enterFullscreen() {
  active.value = true;
  useCssFullscreen.value = false;
  await nextTick();
  const el = overlayRef.value;
  if (!el) {
    return;
  }
  try {
    if (el.requestFullscreen) {
      await el.requestFullscreen();
      return;
    }
  } catch {
    // fall through
  }
  useCssFullscreen.value = true;
}

async function exitDemo() {
  if (!active.value || exiting) {
    return;
  }
  exiting = true;
  try {
    if (document.fullscreenElement) {
      try {
        await document.exitFullscreen();
      } catch {
        // ignore
      }
    }
  } finally {
    active.value = false;
    useCssFullscreen.value = false;
    exiting = false;
  }
}

function onOverlayPointerMove(event: PointerEvent) {
  if (!active.value) {
    return;
  }
  const root = overlayRef.value;
  if (!root) {
    return;
  }
  updateHoleFromEvent(event, root);
}

function onKeyDown(event: KeyboardEvent) {
  if (!active.value) {
    return;
  }
  event.preventDefault();
  void exitDemo();
}

function onOverlayPointerDown() {
  if (!active.value) {
    return;
  }
  void exitDemo();
}

function onFullscreenChange() {
  if (!active.value) {
    return;
  }
  if (!document.fullscreenElement && !useCssFullscreen.value) {
    active.value = false;
  }
}

window.addEventListener("keydown", onKeyDown);
document.addEventListener("fullscreenchange", onFullscreenChange);

onUnmounted(() => {
  window.removeEventListener("keydown", onKeyDown);
  document.removeEventListener("fullscreenchange", onFullscreenChange);
  if (document.fullscreenElement) {
    void document.exitFullscreen().catch(() => undefined);
  }
});
</script>

<template>
  <div class="demo">
    <div class="toolbar">
      <label class="size-row">
        <span class="label">光圈大小</span>
        <input
          v-model.number="size"
          class="slider"
          type="range"
          :min="MIN_SIZE"
          :max="MAX_SIZE"
          step="10"
        />
        <span class="size-value">{{ size }}</span>
      </label>
      <button type="button" class="btn-fullscreen" @click="enterFullscreen">
        全屏体验
      </button>
    </div>
    <p class="hint">
      在预览区移动鼠标查看聚光灯；全屏后与桌面端一致，按任意键或点击退出。
    </p>

    <div
      ref="stageRef"
      class="stage"
      @pointermove="onStagePointerMove"
    >
      <div class="stage-content" aria-hidden="true">
        <h3>1 构建发布</h3>
        <p>预览区模拟文档内容。拖动上方滑块调节光圈，观察双段柔晕过渡。</p>
        <table>
          <thead>
            <tr>
              <th>项目</th>
              <th>说明</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>默认快捷键</td>
              <td>Ctrl+Alt+D</td>
            </tr>
            <tr>
              <td>光圈范围</td>
              <td>100–300，默认 200</td>
            </tr>
            <tr>
              <td>退出</td>
              <td>任意键（任意窗口）</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="mask" :style="maskStyle" />
    </div>

    <div
      v-if="active"
      ref="overlayRef"
      class="ss-overlay"
      :class="{ 'ss-overlay--css': useCssFullscreen }"
      @pointermove="onOverlayPointerMove"
      @pointerdown="onOverlayPointerDown"
    >
      <div class="ss-content" aria-hidden="true">
        <h3>1 构建发布</h3>
        <p>全屏聚光灯演示。移动鼠标聚焦区域；按任意键或点击退出。</p>
        <table>
          <thead>
            <tr>
              <th>项目</th>
              <th>仓库</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>示例 A</td>
              <td>git@example.com:a.git</td>
            </tr>
            <tr>
              <td>示例 B</td>
              <td>git@example.com:b.git</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="mask" :style="maskStyle" />
      <p class="ss-exit-hint">按任意键或点击退出</p>
    </div>
  </div>
</template>

<style scoped>
.demo {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 16px 24px;
}

.size-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 220px;
}

.label {
  font-size: 0.9rem;
  color: var(--text-muted);
  white-space: nowrap;
}

.slider {
  flex: 1;
  min-width: 120px;
  accent-color: var(--accent, #3b82f6);
}

.size-value {
  width: 2.5rem;
  font-variant-numeric: tabular-nums;
  font-size: 0.9rem;
  color: var(--text);
}

.btn-fullscreen {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 8px 16px;
  background: var(--bg-elevated);
  color: var(--text);
  cursor: pointer;
  font-size: 0.95rem;
}

.btn-fullscreen:hover {
  border-color: var(--accent, #3b82f6);
}

.hint {
  margin: 0;
  font-size: 0.9rem;
  color: var(--text-muted);
}

.stage {
  position: relative;
  height: min(420px, 58vh);
  border-radius: 14px;
  overflow: hidden;
  border: 1px solid var(--border);
  background: #f7f8fa;
  color: #1a1a1a;
}

.stage-content,
.ss-content {
  padding: 28px 32px;
  font-size: 0.95rem;
  line-height: 1.6;
}

.stage-content h3,
.ss-content h3 {
  margin: 0 0 10px;
  color: #1a7f4d;
  font-size: 1.25rem;
}

.stage-content p,
.ss-content p {
  margin: 0 0 16px;
  color: #444;
}

.stage-content table,
.ss-content table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.9rem;
}

.stage-content th,
.stage-content td,
.ss-content th,
.ss-content td {
  border: 1px solid #dde2e8;
  padding: 8px 10px;
  text-align: left;
}

.stage-content th,
.ss-content th {
  background: #eef2f6;
}

.mask {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: radial-gradient(
    circle at var(--hole-x) var(--hole-y),
    transparent 0,
    transparent var(--radius),
    rgba(0, 0, 0, var(--inner-start)) var(--radius),
    rgba(0, 0, 0, var(--outer-start)) calc(var(--radius) + var(--inner-feather)),
    rgba(0, 0, 0, var(--mask-alpha))
      calc(var(--radius) + var(--inner-feather) + var(--outer-feather)),
    rgba(0, 0, 0, var(--mask-alpha)) 100%
  );
}

.ss-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: #f7f8fa;
  color: #1a1a1a;
  cursor: none;
}

.ss-overlay--css {
  position: fixed;
}

.ss-exit-hint {
  position: fixed;
  right: 16px;
  bottom: 12px;
  margin: 0;
  font-size: 12px;
  letter-spacing: 0.04em;
  opacity: 0.45;
  pointer-events: none;
  z-index: 2;
  color: #fff;
}
</style>

<script setup lang="ts">
import { nextTick, onUnmounted, ref, shallowRef } from "vue";
import ParallaxSeaside from "@/effects/screensaver/parallax-seaside.vue";
import CoronaSunrise from "@/effects/screensaver/corona-sunrise.vue";
import SnowRelic from "@/effects/screensaver/snow-relic.vue";
import FluorescentClock from "@/effects/screensaver/fluorescent-clock.vue";
import ClassicAnalogClock from "@/effects/screensaver/classic-analog-clock.vue";
import TextCompassClock from "@/effects/screensaver/text-compass-clock.vue";
import type {
  ScreensaverBackground,
  ScreensaverClock,
} from "@/effects/screensaver/types";

const BACKGROUND_OPTIONS: Array<{ id: ScreensaverBackground; label: string }> = [
  { id: "parallax", label: "海景视差" },
  { id: "corona", label: "日冕生辉" },
  { id: "snow", label: "雪国遗踪" },
];

const CLOCK_OPTIONS: Array<{ id: ScreensaverClock; label: string }> = [
  { id: "lcd3d", label: "3D液晶" },
  { id: "analog", label: "经典指针" },
  { id: "compass", label: "文字罗盘" },
  { id: "off", label: "关闭" },
];

const background = shallowRef<ScreensaverBackground>("parallax");
const clock = shallowRef<ScreensaverClock>("lcd3d");
const active = shallowRef(false);
const overlayRef = ref<HTMLElement | null>(null);
const useCssFullscreen = shallowRef(false);

let exiting = false;

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
    // fall through to CSS fullscreen
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

function onKeyDown(event: KeyboardEvent) {
  if (!active.value) {
    return;
  }
  if (event.key === "Escape" || event.key === "F11") {
    event.preventDefault();
    void exitDemo();
  }
}

function onPointerDown() {
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
window.addEventListener("pointerdown", onPointerDown);
document.addEventListener("fullscreenchange", onFullscreenChange);

onUnmounted(() => {
  window.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("pointerdown", onPointerDown);
  document.removeEventListener("fullscreenchange", onFullscreenChange);
  if (document.fullscreenElement) {
    void document.exitFullscreen().catch(() => undefined);
  }
});
</script>

<template>
  <div class="demo">
    <div class="toolbar">
      <div class="group">
        <span class="label">背景</span>
        <div class="effects" role="group" aria-label="背景特效">
          <button
            v-for="item in BACKGROUND_OPTIONS"
            :key="item.id"
            type="button"
            class="chip"
            :class="{ active: background === item.id }"
            @click="background = item.id"
          >
            {{ item.label }}
          </button>
        </div>
      </div>
      <div class="group">
        <span class="label">时钟</span>
        <div class="effects" role="group" aria-label="时钟效果">
          <button
            v-for="item in CLOCK_OPTIONS"
            :key="item.id"
            type="button"
            class="chip"
            :class="{ active: clock === item.id }"
            @click="clock = item.id"
          >
            {{ item.label }}
          </button>
        </div>
      </div>
      <button type="button" class="btn-fullscreen" @click="enterFullscreen">
        全屏体验
      </button>
    </div>
    <p class="hint">
      与桌面端同一套屏保特效。选择背景与时钟后点击「全屏体验」；按 Esc 或 F11 退出。
    </p>
  </div>

  <Teleport to="body">
    <div
      v-if="active"
      ref="overlayRef"
      class="ss-overlay"
      :class="{ 'css-fullscreen': useCssFullscreen }"
    >
      <ParallaxSeaside v-if="background === 'parallax'" />
      <CoronaSunrise v-else-if="background === 'corona'" />
      <SnowRelic v-else-if="background === 'snow'" />
      <FluorescentClock v-if="clock === 'lcd3d'" />
      <ClassicAnalogClock v-else-if="clock === 'analog'" />
      <TextCompassClock v-else-if="clock === 'compass'" />
      <p class="ss-exit-hint">按 ESC、F11 或点击鼠标退出</p>
    </div>
  </Teleport>
</template>

<style scoped>
.demo {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 20px;
  border-radius: 16px;
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  box-shadow: var(--shadow);
}

.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: 16px 20px;
}

.group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}

.label {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-muted);
}

.effects {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.chip {
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-muted);
  border-radius: 999px;
  padding: 8px 14px;
  cursor: pointer;
  font-weight: 600;
  font-size: 0.88rem;
}

.chip.active {
  border-color: var(--brand);
  background: var(--brand-soft);
  color: var(--brand);
}

.btn-fullscreen {
  margin-left: auto;
  align-self: flex-end;
  border: none;
  border-radius: 999px;
  padding: 10px 18px;
  font-weight: 700;
  font-size: 0.9rem;
  cursor: pointer;
  color: #fff;
  background: var(--brand);
}

.btn-fullscreen:hover {
  filter: brightness(1.05);
}

.hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.95rem;
}

.ss-overlay {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: #000;
  color: #fff;
  z-index: 9999;
}

.ss-overlay.css-fullscreen {
  z-index: 100000;
}

.ss-exit-hint {
  position: fixed;
  right: 16px;
  bottom: 12px;
  margin: 0;
  font-size: 12px;
  letter-spacing: 0.04em;
  opacity: 0.35;
  pointer-events: none;
  z-index: 10;
}
</style>

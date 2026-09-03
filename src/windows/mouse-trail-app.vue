<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { MeteorTrail } from "@/effects/meteor-trail";
import { RibbonTrail } from "@/effects/ribbon-trail";
import { GraffitiTrail } from "@/effects/graffiti-trail";
import { DotsTrail } from "@/effects/dots-trail";
import { HeartTrail } from "@/effects/heart-trail";
import { RippleTrail } from "@/effects/ripple-trail";
import {
  DEFAULT_MOUSE_TRAIL_COLORS,
  DEFAULT_MOUSE_TRAIL_PREF,
  isColorableTrailEffect,
  normalizeMouseTrailColors,
  normalizeMouseTrailEffect,
  type MouseTrailColors,
  type MouseTrailEngine,
  type MouseTrailEffect,
  type MouseTrailPref,
} from "@/effects/mouse-trail-types";

interface MonitorBounds {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactor: number;
}

interface CursorPayload {
  x: number;
  y: number;
}

interface TrailSwitchedPayload {
  effect: string;
  label: string;
}

const hostRef = ref<HTMLElement | null>(null);
const bounds = shallowRef<MonitorBounds | null>(null);
const trail = shallowRef<MouseTrailEngine | null>(null);
const effect = shallowRef<MouseTrailEffect>(DEFAULT_MOUSE_TRAIL_PREF.effect);
const colors = shallowRef<MouseTrailColors>({ ...DEFAULT_MOUSE_TRAIL_COLORS });
const toastText = shallowRef("");
let unlistenCursor: (() => void) | undefined;
let unlistenPref: (() => void) | undefined;
let unlistenMonitors: (() => void) | undefined;
let unlistenSwitched: (() => void) | undefined;
let toastTimer: ReturnType<typeof setTimeout> | undefined;
let wasInside = false;
let windowLabel = "";

function isInside(area: MonitorBounds, x: number, y: number): boolean {
  return (
    x >= area.x &&
    x < area.x + area.width &&
    y >= area.y &&
    y < area.y + area.height
  );
}

async function refreshBounds() {
  try {
    if (!windowLabel) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      windowLabel = getCurrentWindow().label;
    }
    bounds.value = await invoke<MonitorBounds>("get_mouse_trail_monitor_bounds", {
      windowLabel,
    });
  } catch {
    bounds.value = {
      x: 0,
      y: 0,
      width: window.innerWidth,
      height: window.innerHeight,
      scaleFactor: window.devicePixelRatio || 1,
    };
  }
  wasInside = false;
  trail.value?.resize();
}

function createEngine(next: MouseTrailEffect, trailColors: MouseTrailColors): MouseTrailEngine | null {
  if (!hostRef.value) {
    return null;
  }
  if (next === "meteor") {
    return new MeteorTrail(hostRef.value, { color: trailColors.meteor });
  }
  if (next === "graffiti") {
    return new GraffitiTrail(hostRef.value);
  }
  if (next === "dots") {
    return new DotsTrail(hostRef.value, { color: trailColors.dots });
  }
  if (next === "heart") {
    return new HeartTrail(hostRef.value, { color: trailColors.heart });
  }
  if (next === "ripple") {
    return new RippleTrail(hostRef.value);
  }
  return new RibbonTrail(hostRef.value);
}

function rebuildEngine(next: MouseTrailEffect, trailColors: MouseTrailColors) {
  trail.value?.destroy();
  trail.value = null;
  wasInside = false;
  effect.value = next;
  colors.value = { ...trailColors };
  const engine = createEngine(next, trailColors);
  if (!engine) {
    return;
  }
  trail.value = engine;
  engine.start();
  requestAnimationFrame(() => {
    trail.value?.resize();
  });
}

function applyColorToEngine(next: MouseTrailEffect, trailColors: MouseTrailColors) {
  const engine = trail.value;
  if (!engine?.setColor || !isColorableTrailEffect(next)) {
    return;
  }
  engine.setColor(trailColors[next]);
}

function applyPref(pref: MouseTrailPref) {
  const normalized = normalizeMouseTrailEffect(pref.effect);
  const normalizedColors = normalizeMouseTrailColors(pref.colors);
  const effectChanged = effect.value !== normalized;
  const colorsChanged =
    colors.value.meteor !== normalizedColors.meteor ||
    colors.value.dots !== normalizedColors.dots ||
    colors.value.heart !== normalizedColors.heart ||
    colors.value.ripple !== normalizedColors.ripple;

  // Cold start: local effect may already match pref (e.g. default ribbon)
  // while trail engine is still null — must rebuild, not only on effect change.
  if (!trail.value || effectChanged) {
    rebuildEngine(normalized, normalizedColors);
    return;
  }
  colors.value = { ...normalizedColors };
  if (colorsChanged) {
    applyColorToEngine(normalized, normalizedColors);
  }
}

function applyCursor(payload: CursorPayload) {
  const area = bounds.value;
  const trailInstance = trail.value;
  if (!area || !trailInstance) {
    return;
  }
  const inside = isInside(area, payload.x, payload.y);
  if (inside) {
    wasInside = true;
    trailInstance.setMouse(
      (payload.x - area.x) / area.scaleFactor,
      (payload.y - area.y) / area.scaleFactor,
    );
    return;
  }
  if (wasInside) {
    wasInside = false;
    trailInstance.leaveScreen();
  }
}

function showSwitchToast(payload: TrailSwitchedPayload) {
  const label = payload.label?.trim();
  if (!label) {
    return;
  }
  toastText.value = `鼠标拖尾特效已切换至${label}`;
  if (toastTimer) {
    clearTimeout(toastTimer);
  }
  toastTimer = setTimeout(() => {
    toastText.value = "";
    toastTimer = undefined;
  }, 1600);
}

onMounted(async () => {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().setIgnoreCursorEvents(true);
  } catch {
    // browser preview
  }

  await nextTick();
  if (!hostRef.value) {
    return;
  }

  await refreshBounds();

  try {
    const pref = await invoke<MouseTrailPref>("get_mouse_trail_pref");
    applyPref(pref);
  } catch {
    rebuildEngine(DEFAULT_MOUSE_TRAIL_PREF.effect, DEFAULT_MOUSE_TRAIL_COLORS);
  }

  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenCursor = await listen<CursorPayload>("app://mouse-trail-cursor", (event) => {
      applyCursor(event.payload);
    });
    unlistenPref = await listen<MouseTrailPref>("app://mouse-trail-pref", (event) => {
      applyPref(event.payload);
    });
    unlistenMonitors = await listen("app://mouse-trail-monitors-changed", () => {
      void refreshBounds();
    });
    unlistenSwitched = await listen<TrailSwitchedPayload>(
      "app://mouse-trail-switched",
      (event) => {
        showSwitchToast(event.payload);
      },
    );
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlistenCursor?.();
  unlistenPref?.();
  unlistenMonitors?.();
  unlistenSwitched?.();
  if (toastTimer) {
    clearTimeout(toastTimer);
  }
  trail.value?.destroy();
  trail.value = null;
});
</script>

<template>
  <div class="overlay-root">
    <div ref="hostRef" class="overlay-host" />
    <div v-if="toastText" class="trail-toast" aria-live="polite">{{ toastText }}</div>
  </div>
</template>

<style scoped>
.overlay-root {
  position: fixed;
  inset: 0;
  overflow: hidden;
  background: transparent;
}

.overlay-host {
  position: absolute;
  inset: 0;
}

.trail-toast {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  z-index: 10;
  pointer-events: none;
  padding: 12px 20px;
  border-radius: 12px;
  background: rgba(20, 24, 32, 0.78);
  color: #fff;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.02em;
  white-space: nowrap;
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.28);
}
</style>

<style>
html.badge-window,
html.badge-window body,
html.badge-window #app {
  margin: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent !important;
}
</style>

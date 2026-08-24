<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { MeteorTrail } from "@/effects/meteor-trail";
import { RibbonTrail } from "@/effects/ribbon-trail";
import { GraffitiTrail } from "@/effects/graffiti-trail";
import { DotsTrail } from "@/effects/dots-trail";
import { HeartTrail } from "@/effects/heart-trail";
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

const hostRef = ref<HTMLElement | null>(null);
const bounds = shallowRef<MonitorBounds | null>(null);
const trail = shallowRef<MouseTrailEngine | null>(null);
const effect = shallowRef<MouseTrailEffect>(DEFAULT_MOUSE_TRAIL_PREF.effect);
const colors = shallowRef<MouseTrailColors>({ ...DEFAULT_MOUSE_TRAIL_COLORS });
let unlistenCursor: (() => void) | undefined;
let unlistenPref: (() => void) | undefined;
let wasInside = false;

function isInside(area: MonitorBounds, x: number, y: number): boolean {
  return (
    x >= area.x &&
    x < area.x + area.width &&
    y >= area.y &&
    y < area.y + area.height
  );
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
    colors.value.heart !== normalizedColors.heart;

  if (effectChanged) {
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

  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const label = getCurrentWindow().label;
    bounds.value = await invoke<MonitorBounds>("get_mouse_trail_monitor_bounds", {
      windowLabel: label,
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
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlistenCursor?.();
  unlistenPref?.();
  trail.value?.destroy();
  trail.value = null;
});
</script>

<template>
  <div ref="hostRef" class="overlay-host" />
</template>

<style scoped>
.overlay-host {
  position: fixed;
  inset: 0;
  overflow: hidden;
  background: transparent;
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

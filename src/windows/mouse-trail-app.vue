<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { MeteorTrail } from "@/effects/meteor-trail";
import { RibbonTrail } from "@/effects/ribbon-trail";
import {
  DEFAULT_MOUSE_TRAIL_PREF,
  normalizeMouseTrailEffect,
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

function createEngine(next: MouseTrailEffect): MouseTrailEngine | null {
  if (!hostRef.value) {
    return null;
  }
  if (next === "meteor") {
    return new MeteorTrail(hostRef.value, { color: "#F8EC85" });
  }
  return new RibbonTrail(hostRef.value);
}

function applyEffect(next: MouseTrailEffect) {
  const normalized = normalizeMouseTrailEffect(next);
  if (trail.value && effect.value === normalized) {
    return;
  }
  trail.value?.destroy();
  trail.value = null;
  wasInside = false;
  effect.value = normalized;
  const engine = createEngine(normalized);
  if (!engine) {
    return;
  }
  trail.value = engine;
  engine.start();
  requestAnimationFrame(() => {
    trail.value?.resize();
  });
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
    applyEffect(normalizeMouseTrailEffect(pref.effect));
  } catch {
    applyEffect(DEFAULT_MOUSE_TRAIL_PREF.effect);
  }

  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenCursor = await listen<CursorPayload>("app://mouse-trail-cursor", (event) => {
      applyCursor(event.payload);
    });
    unlistenPref = await listen<MouseTrailPref>("app://mouse-trail-pref", (event) => {
      applyEffect(normalizeMouseTrailEffect(event.payload.effect));
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

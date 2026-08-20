<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { MeteorTrail } from "@/effects/meteor-trail";

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
const trail = shallowRef<MeteorTrail | null>(null);
let unlistenCursor: (() => void) | undefined;
let unlistenSlots: (() => void) | undefined;
let wasInside = false;

function isInside(area: MonitorBounds, x: number, y: number): boolean {
  return (
    x >= area.x &&
    x < area.x + area.width &&
    y >= area.y &&
    y < area.y + area.height
  );
}

async function loadEffectOptions() {
  try {
    const options = await invoke<{ color: string }>("get_mouse_trail_effect_options");
    trail.value?.setColor(options.color);
  } catch {
    trail.value?.setColor("#F8EC85");
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

  trail.value = new MeteorTrail(hostRef.value, { color: "#F8EC85" });
  await loadEffectOptions();
  trail.value.start();

  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenCursor = await listen<CursorPayload>("app://mouse-trail-cursor", (event) => {
      applyCursor(event.payload);
    });
    unlistenSlots = await listen("app://plugin-slots", () => {
      void loadEffectOptions();
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlistenCursor?.();
  unlistenSlots?.();
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

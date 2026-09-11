<script setup lang="ts">
import { onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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

interface SpotlightPrefPayload {
  size: number;
}

const DEFAULT_SIZE = 200;
const MIN_SIZE = 100;
const MAX_SIZE = 300;
/** Clear core stays at --radius; dual soft rings outside it. */
const INNER_FEATHER_PX = 10;
const OUTER_FEATHER_PX = 800;
const MASK_ALPHA = 0.78;
/** Inner ring starts midway between clear and full mask. */
const INNER_START_ALPHA = MASK_ALPHA * 0.1;
/** Outer ring starts near full mask. */
const OUTER_START_ALPHA = MASK_ALPHA * 0.75;

const holeX = shallowRef("50%");
const holeY = shallowRef("50%");
const radiusPx = shallowRef(DEFAULT_SIZE);

let bounds: MonitorBounds | null = null;
let windowLabel = "";
let unlistenCursor: UnlistenFn | null = null;
let unlistenPref: UnlistenFn | null = null;

function clampSize(raw: number): number {
  if (!Number.isFinite(raw)) {
    return DEFAULT_SIZE;
  }
  return Math.min(MAX_SIZE, Math.max(MIN_SIZE, Math.round(raw)));
}

function applySize(raw: number) {
  radiusPx.value = clampSize(raw);
}

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
    bounds = await invoke<MonitorBounds>("get_spotlight_monitor_bounds", {
      windowLabel,
    });
  } catch {
    bounds = {
      x: 0,
      y: 0,
      width: Math.round(window.innerWidth * (window.devicePixelRatio || 1)),
      height: Math.round(window.innerHeight * (window.devicePixelRatio || 1)),
      scaleFactor: window.devicePixelRatio || 1,
    };
  }
}

function onCursor(payload: CursorPayload) {
  const area = bounds;
  if (!area) {
    return;
  }
  if (!isInside(area, payload.x, payload.y)) {
    holeX.value = "-9999px";
    holeY.value = "-9999px";
    return;
  }
  const scale = area.scaleFactor || 1;
  holeX.value = `${(payload.x - area.x) / scale}px`;
  holeY.value = `${(payload.y - area.y) / scale}px`;
}

onMounted(async () => {
  await refreshBounds();
  try {
    applySize(await invoke<number>("get_spotlight_size"));
  } catch {
    applySize(DEFAULT_SIZE);
  }
  try {
    unlistenCursor = await listen<CursorPayload>("app://spotlight-cursor", (event) => {
      onCursor(event.payload);
    });
  } catch {
    // browser preview
  }
  try {
    unlistenPref = await listen<SpotlightPrefPayload>("app://spotlight-pref", (event) => {
      applySize(event.payload?.size);
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  void unlistenCursor?.();
  void unlistenPref?.();
});
</script>

<template>
  <div
    class="spotlight-root"
    :style="{
      '--hole-x': holeX,
      '--hole-y': holeY,
      '--radius': `${radiusPx}px`,
      '--inner-feather': `${INNER_FEATHER_PX}px`,
      '--outer-feather': `${OUTER_FEATHER_PX}px`,
      '--mask-alpha': String(MASK_ALPHA),
      '--inner-start': String(INNER_START_ALPHA),
      '--outer-start': String(OUTER_START_ALPHA),
    }"
  >
    <div class="mask" />
  </div>
</template>

<style scoped>
.spotlight-root {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  pointer-events: none;
  background: transparent;
}

.mask {
  position: absolute;
  inset: 0;
  /*
   * Clear core = --radius (unchanged spotlight hole).
   * Inner soft: 10px, start alpha midway to mask.
   * Outer soft: 800px, start alpha near full mask.
   */
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
</style>

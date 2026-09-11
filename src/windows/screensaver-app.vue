<script setup lang="ts">
import { onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import ParallaxSeaside from "@/effects/screensaver/parallax-seaside.vue";
import CoronaSunrise from "@/effects/screensaver/corona-sunrise.vue";
import SnowRelic from "@/effects/screensaver/snow-relic.vue";
import FluorescentClock from "@/effects/screensaver/fluorescent-clock.vue";
import ClassicAnalogClock from "@/effects/screensaver/classic-analog-clock.vue";
import TextCompassClock from "@/effects/screensaver/text-compass-clock.vue";
import {
  DEFAULT_SCREENSAVER_PREF,
  normalizeScreensaverPref,
  type ScreensaverPref,
} from "@/effects/screensaver/types";

const pref = shallowRef<ScreensaverPref>({ ...DEFAULT_SCREENSAVER_PREF });

let unlistenPref: UnlistenFn | null = null;

async function exitScreensaver() {
  try {
    await invoke("hide_screensaver");
  } catch {
    // browser preview
  }
}

function onKeyDown(event: KeyboardEvent) {
  event.preventDefault();
  void exitScreensaver();
}

function onPointerDown() {
  void exitScreensaver();
}

onMounted(async () => {
  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("pointerdown", onPointerDown);
  try {
    const raw = await invoke<unknown>("get_screensaver_effect_pref");
    pref.value = normalizeScreensaverPref(raw);
  } catch {
    pref.value = { ...DEFAULT_SCREENSAVER_PREF };
  }
  try {
    unlistenPref = await listen<unknown>("app://screensaver-pref", (event) => {
      pref.value = normalizeScreensaverPref(event.payload);
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("pointerdown", onPointerDown);
  void unlistenPref?.();
});
</script>

<template>
  <div class="screensaver-root">
    <ParallaxSeaside v-if="pref.background === 'parallax'" />
    <CoronaSunrise v-else-if="pref.background === 'corona'" />
    <SnowRelic v-else-if="pref.background === 'snow'" />
    <FluorescentClock v-if="pref.clock === 'lcd3d'" />
    <ClassicAnalogClock v-else-if="pref.clock === 'analog'" />
    <TextCompassClock v-else-if="pref.clock === 'compass'" />
    <p class="hint">按任意键或点击鼠标退出</p>
  </div>
</template>

<style scoped>
.screensaver-root {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: #000;
  color: #fff;
}

.hint {
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

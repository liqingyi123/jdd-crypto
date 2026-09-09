<script setup lang="ts">
import { onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import ParallaxSeaside from "@/effects/screensaver/parallax-seaside.vue";
import CoronaSunrise from "@/effects/screensaver/corona-sunrise.vue";
import SnowRelic from "@/effects/screensaver/snow-relic.vue";
import FluorescentClock from "@/effects/screensaver/fluorescent-clock.vue";
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
  if (event.key === "Escape") {
    event.preventDefault();
    void exitScreensaver();
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onKeyDown);
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
  void unlistenPref?.();
});
</script>

<template>
  <div class="screensaver-root">
    <ParallaxSeaside v-if="pref.background === 'parallax'" />
    <CoronaSunrise v-else-if="pref.background === 'corona'" />
    <SnowRelic v-else-if="pref.background === 'snow'" />
    <FluorescentClock v-if="pref.clock === 'lcd3d'" />
    <p class="hint">按 ESC 退出</p>
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

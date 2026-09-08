<script setup lang="ts">
import { onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import ParallaxSeaside from "@/effects/screensaver/parallax-seaside.vue";

type ScreensaverEffect = "parallax";

const effect = shallowRef<ScreensaverEffect>("parallax");

let unlistenEffect: UnlistenFn | null = null;

function normalizeEffect(_raw: unknown): ScreensaverEffect {
  return "parallax";
}

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
    const pref = await invoke<{ effect: string }>("get_screensaver_effect_pref");
    effect.value = normalizeEffect(pref.effect);
  } catch {
    effect.value = "parallax";
  }
  try {
    unlistenEffect = await listen<string>("app://screensaver-effect", (event) => {
      effect.value = normalizeEffect(event.payload);
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeyDown);
  void unlistenEffect?.();
});
</script>

<template>
  <div class="screensaver-root">
    <ParallaxSeaside v-if="effect === 'parallax'" />
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

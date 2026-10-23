<script setup lang="ts">
import { onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

interface MiniWindowPrefs {
  autoHide: boolean;
  alwaysOnTop: boolean;
  contentOpacity: number;
  backgroundOpacity: number;
  browsing: boolean;
}

const contentOpacity = shallowRef(1);
const backgroundOpacity = shallowRef(1);

let unlistenPrefs: UnlistenFn | undefined;
let unlistenBlur: (() => void) | undefined;
let blurArmed = false;
let armTimer: ReturnType<typeof setTimeout> | undefined;

function applyPrefs(prefs: MiniWindowPrefs) {
  contentOpacity.value = Number(prefs.contentOpacity) || 1;
  backgroundOpacity.value = Number(prefs.backgroundOpacity) || 1;
}

async function hideWindow() {
  await invoke("hide_mini_opacity_bubble").catch(() => undefined);
}

async function onContentOpacity(value: number) {
  contentOpacity.value = value / 100;
  await invoke("set_mini_window_content_opacity", {
    opacity: contentOpacity.value,
  }).catch(() => undefined);
}

async function onBackgroundOpacity(value: number) {
  backgroundOpacity.value = value / 100;
  await invoke("set_mini_window_background_opacity", {
    opacity: backgroundOpacity.value,
  }).catch(() => undefined);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    void hideWindow();
  }
}

function clearArmTimer() {
  if (armTimer !== undefined) {
    clearTimeout(armTimer);
    armTimer = undefined;
  }
}

function armBlurDismiss() {
  clearArmTimer();
  blurArmed = false;
  armTimer = setTimeout(() => {
    blurArmed = true;
  }, 280);
}

onMounted(async () => {
  armBlurDismiss();
  window.addEventListener("keydown", onKeydown);
  try {
    const prefs = await invoke<MiniWindowPrefs>("get_mini_window_prefs");
    applyPrefs(prefs);
  } catch {
    // browser preview
  }
  try {
    const win = getCurrentWindow();
    unlistenBlur = await win.onFocusChanged(({ payload: focused }) => {
      if (!focused && blurArmed) {
        void hideWindow();
      }
    });
    unlistenPrefs = await listen<MiniWindowPrefs>("mini-window://prefs", (event) => {
      applyPrefs(event.payload);
      armBlurDismiss();
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  clearArmTimer();
  window.removeEventListener("keydown", onKeydown);
  unlistenBlur?.();
  void unlistenPrefs?.();
});
</script>

<template>
  <div class="opacity-bubble">
    <label class="opacity-row">
      <span>整体透明度</span>
      <input
        type="range"
        min="20"
        max="100"
        :value="Math.round(backgroundOpacity * 100)"
        @input="onBackgroundOpacity(Number(($event.target as HTMLInputElement).value))"
      />
    </label>
    <label class="opacity-row">
      <span>内容透明度</span>
      <input
        type="range"
        min="20"
        max="100"
        :value="Math.round(contentOpacity * 100)"
        @input="onContentOpacity(Number(($event.target as HTMLInputElement).value))"
      />
    </label>
  </div>
</template>

<style scoped>
.opacity-bubble {
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  padding: 12px;
  border-radius: 10px;
  background: #2f2924;
  border: 1px solid rgba(255, 214, 186, 0.2);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35);
  display: flex;
  flex-direction: column;
  gap: 10px;
  color: #f3d9c4;
  user-select: none;
}

.opacity-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
}

.opacity-row input[type="range"] {
  width: 100%;
  accent-color: #f0c9ad;
}
</style>

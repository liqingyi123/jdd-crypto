<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import OverlayToast from "@/components/overlay-toast.vue";
import { useOverlayToast } from "@/composables/use-overlay-toast";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

interface OverlayToastPayload {
  message: string;
}

const { toastText, showToast } = useOverlayToast();
let unlisten: (() => void) | undefined;

onMounted(async () => {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().setIgnoreCursorEvents(true);
  } catch {
    // browser preview
  }

  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<OverlayToastPayload>("app://overlay-toast", (event) => {
      showToast(event.payload?.message ?? "");
    });
    const pending = await invoke<string | null>("take_pending_overlay_toast").catch(
      () => null,
    );
    if (pending) {
      showToast(pending);
    }
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <div class="toast-root">
    <OverlayToast :text="toastText" />
  </div>
</template>

<style scoped>
.toast-root {
  position: relative;
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  overflow: visible;
  background: transparent;
  pointer-events: none;
  user-select: none;
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

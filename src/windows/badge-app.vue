<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSystemTheme } from "@/composables/use-system-theme";
import { DEFAULT_BADGE_SIZE } from "@/constants/badge";

const appIcon = "/app-icon.png";

useSystemTheme();

const pointerOrigin = ref<{ x: number; y: number } | null>(null);
const didDrag = shallowRef(false);
const badgeSize = shallowRef(DEFAULT_BADGE_SIZE);
const appVersion = shallowRef("");
let unlistenSize: (() => void) | undefined;

const badgeStyle = computed(() => {
  const size = badgeSize.value;
  const pad = Math.round((size * 12) / 96);
  const orb = Math.max(16, size - pad * 2);
  const radius = Math.max(4, Math.round((orb * 12) / 72));
  return {
    "--badge-pad": `${pad}px`,
    "--badge-orb": `${orb}px`,
    "--badge-radius": `${radius}px`,
  };
});

const badgeTip = computed(() => {
  const title = appVersion.value
    ? `多多解密 v${appVersion.value}`
    : "多多解密";
  return [
    title,
    "左键：打开主界面",
    "中键：打开 Host 管理",
    "右键：打开菜单",
    "可拖动",
  ].join("\n");
});

onMounted(async () => {
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    appVersion.value = await getVersion();
  } catch {
    // browser preview
  }
  try {
    badgeSize.value = await invoke<number>("get_badge_size");
  } catch {
    // browser preview
  }
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenSize = await listen<number>("app://badge-size", (event) => {
      badgeSize.value = event.payload;
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlistenSize?.();
});

async function openMain() {
  if (didDrag.value) {
    return;
  }
  await invoke("navigate_main", { route: "/" });
}

async function openMenu(event: MouseEvent) {
  event.preventDefault();
  await invoke("popup_app_menu").catch(() => undefined);
}

async function openHosts(event: MouseEvent) {
  if (event.button !== 1) {
    return;
  }
  event.preventDefault();
  await invoke("open_hosts_window").catch(() => undefined);
}

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  pointerOrigin.value = { x: event.clientX, y: event.clientY };
  didDrag.value = false;
}

async function onPointerMove(event: PointerEvent) {
  if (!pointerOrigin.value) {
    return;
  }

  if (!didDrag.value) {
    const dx = event.clientX - pointerOrigin.value.x;
    const dy = event.clientY - pointerOrigin.value.y;
    if (dx * dx + dy * dy < 25) {
      return;
    }
    didDrag.value = true;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    } catch {
      // browser preview
    }
  }
}

function onPointerUp() {
  pointerOrigin.value = null;
}
</script>

<template>
  <div class="badge-root" :style="badgeStyle">
    <button
      class="orb"
      type="button"
      :title="badgeTip"
      @click="openMain"
      @auxclick="openHosts"
      @contextmenu="openMenu"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
    >
      <img :src="appIcon" alt="多多解密" />
    </button>
  </div>
</template>

<style scoped>
.badge-root {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: flex-start;
  padding: var(--badge-pad, 12px);
  background: transparent;
  user-select: none;
}

.orb {
  width: var(--badge-orb, 72px);
  height: var(--badge-orb, 72px);
  flex-shrink: 0;
  padding: 0;
  border: 0;
  border-radius: var(--badge-radius, 12px);
  overflow: visible;
  background: transparent;
}

.orb img {
  display: block;
  width: 100%;
  height: 100%;
  border-radius: var(--badge-radius, 12px);
  object-fit: cover;
  pointer-events: none;
  filter: drop-shadow(0 0 4px rgba(255, 120, 110, 0.35))
    drop-shadow(0 0 10px rgba(220, 70, 60, 0.1));
}
</style>

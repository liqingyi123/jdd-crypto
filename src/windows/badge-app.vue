<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useClipboardPrompt } from "@/composables/use-clipboard-prompt";
import { useBadgePromptPlacement } from "@/composables/use-badge-prompt-placement";
import { useSystemTheme } from "@/composables/use-system-theme";
import appIcon from "@/assets/app-icon.png";

useSystemTheme();
const { clipboardStore, accept, dismiss } = useClipboardPrompt();

const promptOpen = computed(() => clipboardStore.candidate !== null);
const kindLabel = computed(() => {
  const kind = clipboardStore.candidate?.kind;
  if (kind === "maybe_cipher") return "疑似密文";
  if (kind === "maybe_plain") return "疑似明文";
  return "剪贴板文本";
});

const pointerOrigin = ref<{ x: number; y: number } | null>(null);
const didDrag = shallowRef(false);
const badgeSize = shallowRef(68);
let unlistenSize: (() => void) | undefined;

const { placementClass, beginExpandedDrag, moveExpandedDrag, endExpandedDrag } =
  useBadgePromptPlacement({
    promptOpen,
    badgeSize,
  });

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

onMounted(async () => {
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

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  pointerOrigin.value = { x: event.clientX, y: event.clientY };
  didDrag.value = false;
  if (promptOpen.value) {
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }
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
    if (promptOpen.value) {
      beginExpandedDrag(event);
      moveExpandedDrag(event);
      return;
    }
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    } catch {
      // browser preview
    }
    return;
  }

  if (promptOpen.value) {
    moveExpandedDrag(event);
  }
}

function onPointerUp(event: PointerEvent) {
  const shouldSettle = didDrag.value && promptOpen.value;
  pointerOrigin.value = null;
  if (shouldSettle) {
    endExpandedDrag();
  }
  const target = event.currentTarget as HTMLElement;
  if (target.hasPointerCapture(event.pointerId)) {
    target.releasePointerCapture(event.pointerId);
  }
}
</script>

<template>
  <div
    class="badge-root"
    :class="[{ expanded: promptOpen }, placementClass]"
    :style="badgeStyle"
  >
    <button
      class="orb"
      type="button"
      title="多多解密：左键打开主界面，右键打开菜单，可拖动"
      @click="openMain"
      @contextmenu="openMenu"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
    >
      <img :src="appIcon" alt="多多解密" />
    </button>

    <div v-if="promptOpen" class="prompt">
      <p>{{ kindLabel }}，是否立即处理？</p>
      <div class="actions">
        <button type="button" @click="accept('encrypt')">加密</button>
        <button type="button" @click="accept('decrypt')">解密</button>
        <button type="button" class="ghost" @click="dismiss">忽略</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.badge-root {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: var(--badge-pad, 12px);
  background: transparent;
  user-select: none;
}

.badge-root.prompt-left {
  flex-direction: row-reverse;
}

.badge-root.prompt-up {
  align-items: flex-end;
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
  image-rendering: pixelated;
  pointer-events: none;
  filter: drop-shadow(0 0 4px rgba(255, 120, 110, 0.35))
    drop-shadow(0 0 10px rgba(220, 70, 60, 0.1));
}

.prompt {
  flex: 1;
  padding: 8px 10px;
  border-radius: 12px;
  background: var(--bg-elevated);
  color: var(--text);
  box-shadow: var(--shadow);
  font-size: 12px;
}

.prompt p {
  margin: 0 0 8px;
}

.actions {
  display: flex;
  gap: 6px;
}

.actions button {
  border: 0;
  border-radius: 8px;
  padding: 4px 8px;
  background: var(--brand);
  color: #fff;
}

.actions .ghost {
  background: var(--bg-muted);
  color: var(--text);
}
</style>

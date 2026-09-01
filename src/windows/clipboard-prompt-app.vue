<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch } from "vue";
import { useClipboardPrompt } from "@/composables/use-clipboard-prompt";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

let suppressBlurDismiss = false;

function onSuppressBlurDismiss() {
  suppressBlurDismiss = true;
}

const { clipboardStore, accept, dismiss } = useClipboardPrompt({
  fetchOnMount: true,
  hideWindowOnClose: true,
  onSuppressBlurDismiss,
});

const visible = computed(() => clipboardStore.candidate !== null);

let blurArmed = false;
let hadFocus = false;
let armTimer: ReturnType<typeof setTimeout> | undefined;
let unlistenFocus: (() => void) | undefined;

function clearArmTimer() {
  if (armTimer !== undefined) {
    clearTimeout(armTimer);
    armTimer = undefined;
  }
}

function armBlurDismiss() {
  clearArmTimer();
  blurArmed = false;
  hadFocus = false;
  suppressBlurDismiss = false;
  // 询问窗默认不抢焦点；延长武装时间，避免刚显示就被系统焦点抖动关掉
  armTimer = setTimeout(() => {
    blurArmed = true;
    armTimer = undefined;
  }, 600);
}

function onActionPointerDown() {
  // pointerdown 早于 click/blur，先锁住失焦关闭，避免点「解密」时窗体被先 dismiss
  suppressBlurDismiss = true;
}

onMounted(async () => {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) {
        hadFocus = true;
        return;
      }
      if (suppressBlurDismiss || !hadFocus || !blurArmed || !clipboardStore.candidate) {
        return;
      }
      void dismiss();
    });
  } catch {
    // browser preview
  }
  if (visible.value) {
    armBlurDismiss();
  }
});

onUnmounted(() => {
  clearArmTimer();
  unlistenFocus?.();
});

watch(visible, (open) => {
  if (open) {
    armBlurDismiss();
    return;
  }
  clearArmTimer();
  blurArmed = false;
  hadFocus = false;
});
</script>

<template>
  <div v-if="visible" class="prompt-root">
    <div class="prompt">
      <p>你复制了这段文本，是想要？</p>
      <div class="actions">
        <button type="button" @pointerdown="onActionPointerDown" @click="accept('encrypt')">
          加密
        </button>
        <button type="button" @pointerdown="onActionPointerDown" @click="accept('decrypt')">
          解密
        </button>
        <button
          type="button"
          class="ghost"
          @pointerdown="onActionPointerDown"
          @click="dismiss"
        >
          忽略
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.prompt-root {
  width: 100%;
  height: 100%;
  padding: 14px;
  box-sizing: border-box;
  background: transparent;
  user-select: none;
  overflow: visible;
}

.prompt {
  height: 100%;
  padding: 8px 10px;
  border-radius: 12px;
  border: 1px solid rgba(239, 68, 68, 0.85);
  background: var(--bg-elevated);
  color: var(--text);
  font-size: 12px;
  box-sizing: border-box;
  box-shadow:
    0 0 0 1px rgba(239, 68, 68, 0.35),
    0 0 10px rgba(239, 68, 68, 0.45),
    0 0 18px rgba(220, 38, 38, 0.3);
  animation: prompt-breathe 1.4s ease-in-out infinite;
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
  cursor: pointer;
}

.actions .ghost {
  background: var(--bg-muted);
  color: var(--text);
}

@keyframes prompt-breathe {
  0%,
  100% {
    border-color: rgba(239, 68, 68, 0.55);
    box-shadow:
      0 0 0 1px rgba(239, 68, 68, 0.2),
      0 0 6px rgba(239, 68, 68, 0.28),
      0 0 12px rgba(220, 38, 38, 0.16);
  }
  50% {
    border-color: rgba(248, 113, 113, 1);
    box-shadow:
      0 0 0 1px rgba(248, 113, 113, 0.55),
      0 0 14px rgba(239, 68, 68, 0.75),
      0 0 28px rgba(220, 38, 38, 0.45);
  }
}
</style>

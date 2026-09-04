import { onUnmounted, shallowRef } from "vue";

const DEFAULT_DURATION_MS = 1600;

export function useOverlayToast(defaultDurationMs = DEFAULT_DURATION_MS) {
  const toastText = shallowRef("");
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  function clearToastTimer() {
    if (toastTimer !== undefined) {
      clearTimeout(toastTimer);
      toastTimer = undefined;
    }
  }

  function showToast(message: string, durationMs = defaultDurationMs) {
    const text = message.trim();
    if (!text) {
      return;
    }
    clearToastTimer();
    toastText.value = text;
    toastTimer = setTimeout(() => {
      toastText.value = "";
      toastTimer = undefined;
    }, durationMs);
  }

  function clearToast() {
    clearToastTimer();
    toastText.value = "";
  }

  onUnmounted(() => {
    clearToastTimer();
  });

  return {
    toastText,
    showToast,
    clearToast,
  };
}

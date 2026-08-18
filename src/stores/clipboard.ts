import { defineStore } from "pinia";
import { ref } from "vue";

export type ClipboardKind = "maybe_cipher" | "maybe_plain" | "unknown";

export interface ClipboardCandidate {
  text: string;
  kind: ClipboardKind | string;
}

export const useClipboardStore = defineStore("clipboard", () => {
  const watchEnabled = ref(true);
  const candidate = ref<ClipboardCandidate | null>(null);

  function setWatchEnabled(enabled: boolean) {
    watchEnabled.value = enabled;
  }

  function setCandidate(next: ClipboardCandidate | null) {
    candidate.value = next;
  }

  function clearCandidate() {
    candidate.value = null;
  }

  return {
    watchEnabled,
    candidate,
    setWatchEnabled,
    setCandidate,
    clearCandidate,
  };
});

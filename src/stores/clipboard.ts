import { defineStore } from "pinia";
import { ref } from "vue";

export type ClipboardKind = "maybe_cipher" | "maybe_plain" | "unknown";

export interface ClipboardCandidate {
  text: string;
  kind: ClipboardKind;
}

function normalizeKind(kind: string): ClipboardKind {
  if (kind === "maybe_cipher" || kind === "maybe_plain" || kind === "unknown") {
    return kind;
  }
  return "unknown";
}

export const useClipboardStore = defineStore("clipboard", () => {
  const watchEnabled = ref(true);
  const candidate = ref<ClipboardCandidate | null>(null);

  function setWatchEnabled(enabled: boolean) {
    watchEnabled.value = enabled;
  }

  function setCandidate(next: ClipboardCandidate | null) {
    if (!next) {
      candidate.value = null;
      return;
    }
    candidate.value = {
      text: next.text,
      kind: normalizeKind(String(next.kind)),
    };
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

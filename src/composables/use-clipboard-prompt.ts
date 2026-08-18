import { onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useClipboardStore, type ClipboardCandidate } from "@/stores/clipboard";

export function useClipboardPrompt() {
  const clipboardStore = useClipboardStore();
  let unlisten: (() => void) | undefined;
  let unlistenFollow: (() => void) | undefined;

  onMounted(async () => {
    try {
      const enabled = await invoke<boolean>("get_clipboard_watch");
      clipboardStore.setWatchEnabled(enabled);
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen<ClipboardCandidate>("clipboard://candidate", (event) => {
        if (!clipboardStore.watchEnabled) {
          return;
        }
        clipboardStore.setCandidate(event.payload);
      });
      unlistenFollow = await listen<boolean>("app://mouse-follow", (event) => {
        if (event.payload) {
          clipboardStore.clearCandidate();
        }
      });
    } catch {
      // not running inside Tauri
    }
  });

  onUnmounted(() => {
    unlisten?.();
    unlistenFollow?.();
  });

  async function accept(mode: "encrypt" | "decrypt") {
    const candidate = clipboardStore.candidate;
    if (!candidate) {
      return;
    }
    await invoke("navigate_main", {
      route: "/",
      mode,
      text: candidate.text,
    });
    clipboardStore.clearCandidate();
  }

  async function dismiss() {
    clipboardStore.clearCandidate();
  }

  return {
    clipboardStore,
    accept,
    dismiss,
  };
}

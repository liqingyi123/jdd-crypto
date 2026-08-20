import { onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useClipboardStore, type ClipboardCandidate } from "@/stores/clipboard";

export function useClipboardPrompt(options?: {
  fetchOnMount?: boolean;
  hideWindowOnClose?: boolean;
}) {
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
      if (options?.fetchOnMount) {
        const existing = await invoke<ClipboardCandidate | null>(
          "get_clipboard_candidate",
        ).catch(() => null);
        if (existing && clipboardStore.watchEnabled) {
          clipboardStore.setCandidate(existing);
        }
      }
    } catch {
      // not running inside Tauri
    }
  });

  onUnmounted(() => {
    unlisten?.();
    unlistenFollow?.();
  });

  async function closePromptWindow() {
    if (!options?.hideWindowOnClose) {
      return;
    }
    await invoke("hide_clipboard_prompt").catch(() => undefined);
  }

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
    await closePromptWindow();
  }

  async function dismiss() {
    clipboardStore.clearCandidate();
    await closePromptWindow();
  }

  return {
    clipboardStore,
    accept,
    dismiss,
  };
}

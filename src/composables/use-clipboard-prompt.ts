import { onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useClipboardStore, type ClipboardCandidate } from "@/stores/clipboard";

export function useClipboardPrompt(options?: {
  fetchOnMount?: boolean;
  hideWindowOnClose?: boolean;
  /** Set true before any async work so blur-dismiss cannot race with button clicks. */
  onSuppressBlurDismiss?: () => void;
}) {
  const clipboardStore = useClipboardStore();
  let unlisten: (() => void) | undefined;
  let unlistenFollow: (() => void) | undefined;

  onMounted(async () => {
    try {
      const enabled = await invoke<boolean>("get_clipboard_watch").catch(() => true);
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
    options?.onSuppressBlurDismiss?.();
    const text = clipboardStore.candidate?.text ?? null;
    // Rust 端打开气泡/主窗；text 作为 last_candidate 被失焦清掉时的兜底
    const ok = await invoke<boolean>("accept_clipboard_action", { mode, text }).catch(
      () => false,
    );
    clipboardStore.clearCandidate();
    if (!ok) {
      await invoke("clear_clipboard_dedup").catch(() => undefined);
      await closePromptWindow();
    }
  }

  async function dismiss() {
    options?.onSuppressBlurDismiss?.();
    clipboardStore.clearCandidate();
    await closePromptWindow();
  }

  return {
    clipboardStore,
    accept,
    dismiss,
  };
}

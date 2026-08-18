import type { PluginHostApi } from "./types";

/**
 * iframe + postMessage sandbox placeholder.
 * High-risk capabilities must go through the host API, never direct OS access.
 */
export function createSandboxApi(
  onMessage: (channel: string, payload: unknown) => void,
): PluginHostApi {
  return {
    registerCryptoOption(option) {
      onMessage("crypto-option", option);
    },
    registerEditor(editor) {
      onMessage("editor", editor);
    },
    registerOverlayEffect(effect) {
      onMessage("overlay-effect", effect);
    },
    async requestOverlay() {
      onMessage("request-overlay", {});
      return false;
    },
  };
}

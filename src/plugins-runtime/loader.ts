import type { PluginHostApi, PluginManifest } from "./types";
import { createSandboxApi } from "./sandbox";

export interface LoadResult {
  manifest: PluginManifest;
  loaded: boolean;
  reason?: string;
}

/**
 * Phase 0: only register declared contributions.
 * Actual `import(entry)` inside an iframe sandbox lands in a later iteration.
 */
export async function loadPlugin(
  manifest: PluginManifest,
  api: PluginHostApi = createSandboxApi(() => {}),
): Promise<LoadResult> {
  if (!manifest.enabled) {
    return { manifest, loaded: false, reason: "disabled" };
  }

  const options = manifest.contributes.cryptoOptions ?? [];
  for (const option of options) {
    api.registerCryptoOption?.(option);
  }
  if (manifest.contributes.editor) {
    api.registerEditor?.({
      id: manifest.id,
      label: manifest.name,
    });
  }
  if (manifest.contributes.overlayEffect) {
    api.registerOverlayEffect?.({
      id: manifest.id,
      label: manifest.name,
    });
  }

  return { manifest, loaded: true };
}

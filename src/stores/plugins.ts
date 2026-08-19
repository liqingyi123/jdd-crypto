import { defineStore } from "pinia";
import { ref } from "vue";
import type { CryptoOptionContribution, PluginManifest } from "@/plugins-runtime/types";

export const usePluginsStore = defineStore("plugins", () => {
  const manifests = ref<PluginManifest[]>([]);
  const cryptoOptions = ref<CryptoOptionContribution[]>([]);
  const editors = ref<Array<{ id: string; label: string }>>([]);
  const overlayEffects = ref<Array<{ id: string; label: string }>>([]);

  function setManifests(next: PluginManifest[]) {
    manifests.value = next;
  }

  function registerCryptoOption(option: CryptoOptionContribution) {
    if (cryptoOptions.value.some((item) => item.id === option.id)) {
      return;
    }
    cryptoOptions.value = [...cryptoOptions.value, option];
  }

  function registerEditor(editor: { id: string; label: string }) {
    if (editors.value.some((item) => item.id === editor.id)) {
      return;
    }
    editors.value = [...editors.value, editor];
  }

  function registerOverlayEffect(effect: { id: string; label: string }) {
    if (overlayEffects.value.some((item) => item.id === effect.id)) {
      return;
    }
    overlayEffects.value = [...overlayEffects.value, effect];
  }

  return {
    manifests,
    cryptoOptions,
    editors,
    overlayEffects,
    setManifests,
    registerCryptoOption,
    registerEditor,
    registerOverlayEffect,
  };
});

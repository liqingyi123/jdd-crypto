import { defineStore } from "pinia";
import { ref } from "vue";

export type PendingCryptoMode = "encrypt" | "decrypt" | "auto";

export interface PendingCryptoPayload {
  text: string;
  mode: PendingCryptoMode;
}

function normalizeMode(mode: string): PendingCryptoMode {
  if (mode === "encrypt" || mode === "decrypt" || mode === "auto") {
    return mode;
  }
  return "auto";
}

export const useAppStore = defineStore("app", () => {
  const pendingCrypto = ref<PendingCryptoPayload | null>(null);

  function setPendingCrypto(payload: { text: string; mode: string } | null) {
    if (!payload) {
      pendingCrypto.value = null;
      return;
    }
    pendingCrypto.value = {
      text: payload.text,
      mode: normalizeMode(String(payload.mode)),
    };
  }

  return {
    pendingCrypto,
    setPendingCrypto,
  };
});

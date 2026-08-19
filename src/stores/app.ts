import { defineStore } from "pinia";
import { ref } from "vue";

export interface PendingCryptoPayload {
  text: string;
  mode: "encrypt" | "decrypt" | string;
}

export const useAppStore = defineStore("app", () => {
  const pendingCrypto = ref<PendingCryptoPayload | null>(null);

  function setPendingCrypto(payload: PendingCryptoPayload | null) {
    pendingCrypto.value = payload;
  }

  return {
    pendingCrypto,
    setPendingCrypto,
  };
});

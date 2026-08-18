import { defineStore } from "pinia";
import { ref } from "vue";

export interface PendingCryptoPayload {
  text: string;
  mode: "encrypt" | "decrypt" | string;
}

export const useAppStore = defineStore("app", () => {
  const windowLabel = ref("main");
  const pendingCrypto = ref<PendingCryptoPayload | null>(null);

  function setWindowLabel(label: string) {
    windowLabel.value = label;
  }

  function setPendingCrypto(payload: PendingCryptoPayload | null) {
    pendingCrypto.value = payload;
  }

  return {
    windowLabel,
    pendingCrypto,
    setWindowLabel,
    setPendingCrypto,
  };
});

import { defineStore } from "pinia";
import { ref, watch } from "vue";
import { DEFAULT_AES_CODE } from "@/utils/aes-enum";
import type { AesOpType } from "@/services/aes-ops";

const STORAGE_KEY = "jdd-crypto-workspace";
const HISTORY_LIMIT = 50;

export interface CryptoHistoryItem {
  id: string;
  type: AesOpType;
  at: number;
  text: string;
  aesCode: string;
  key: string;
  iv: string;
  result: string;
}

interface PersistedWorkspace {
  aesCode: string;
  customKey: string;
  customIv: string;
  history: CryptoHistoryItem[];
}

function loadPersisted(): PersistedWorkspace {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return {
        aesCode: DEFAULT_AES_CODE,
        customKey: "",
        customIv: "",
        history: [],
      };
    }
    const parsed = JSON.parse(raw) as Partial<PersistedWorkspace>;
    const history = Array.isArray(parsed.history)
      ? parsed.history.map((entry, index) => normalizeHistoryItem(entry, index))
      : [];
    return {
      aesCode: typeof parsed.aesCode === "string" ? parsed.aesCode : DEFAULT_AES_CODE,
      customKey: typeof parsed.customKey === "string" ? parsed.customKey : "",
      customIv: typeof parsed.customIv === "string" ? parsed.customIv : "",
      history,
    };
  } catch {
    return {
      aesCode: DEFAULT_AES_CODE,
      customKey: "",
      customIv: "",
      history: [],
    };
  }
}

function normalizeHistoryItem(
  entry: Partial<CryptoHistoryItem>,
  index: number,
): CryptoHistoryItem {
  return {
    id:
      typeof entry.id === "string" && entry.id
        ? entry.id
        : `${entry.at ?? Date.now()}-${index}-${Math.random().toString(36).slice(2, 8)}`,
    type: entry.type === "encrypt" || entry.type === "decrypt" || entry.type === "tokv"
      ? entry.type
      : "decrypt",
    at: typeof entry.at === "number" ? entry.at : Date.now(),
    text: typeof entry.text === "string" ? entry.text : "",
    aesCode: typeof entry.aesCode === "string" ? entry.aesCode : DEFAULT_AES_CODE,
    key: typeof entry.key === "string" ? entry.key : "",
    iv: typeof entry.iv === "string" ? entry.iv : "",
    result: typeof entry.result === "string" ? entry.result : "",
  };
}

function isSameHistoryEntry(
  a: Pick<CryptoHistoryItem, "type" | "text" | "aesCode" | "key" | "iv">,
  b: Pick<CryptoHistoryItem, "type" | "text" | "aesCode" | "key" | "iv">,
): boolean {
  return (
    a.type === b.type &&
    a.text === b.text &&
    a.aesCode === b.aesCode &&
    a.key === b.key &&
    a.iv === b.iv
  );
}

export const useCryptoWorkspaceStore = defineStore("crypto-workspace", () => {
  const initial = loadPersisted();
  const aesCode = ref(initial.aesCode || DEFAULT_AES_CODE);
  const customKey = ref(initial.customKey);
  const customIv = ref(initial.customIv);
  const history = ref<CryptoHistoryItem[]>(initial.history);

  function persist() {
    const payload: PersistedWorkspace = {
      aesCode: aesCode.value,
      customKey: customKey.value,
      customIv: customIv.value,
      history: history.value.slice(0, HISTORY_LIMIT),
    };
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
    } catch {
      // quota / private mode
    }
  }

  watch([aesCode, customKey, customIv, history], persist, { deep: true });

  function setAesCode(code: string) {
    aesCode.value = code;
  }

  function setCustomKey(value: string) {
    customKey.value = value;
  }

  function setCustomIv(value: string) {
    customIv.value = value;
  }

  /** Remember key/iv used with custom (or when auto landed on custom). */
  function rememberCustom(key: string, iv: string) {
    if (!key || !iv) {
      return;
    }
    customKey.value = key;
    customIv.value = iv;
  }

  function pushHistory(item: Omit<CryptoHistoryItem, "id" | "at"> & { at?: number }) {
    const next: CryptoHistoryItem = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      at: item.at ?? Date.now(),
      type: item.type,
      text: item.text,
      aesCode: item.aesCode,
      key: item.key,
      iv: item.iv,
      result: item.result,
    };
    const deduped = history.value.filter((entry) => !isSameHistoryEntry(entry, next));
    history.value = [next, ...deduped].slice(0, HISTORY_LIMIT);
  }

  function clearHistory() {
    history.value = [];
  }

  function removeHistory(id: string) {
    const next = history.value.filter((entry) => entry.id !== id);
    if (next.length === history.value.length) {
      return;
    }
    history.value = next;
  }

  function removeHistoryAt(index: number) {
    if (index < 0 || index >= history.value.length) {
      return;
    }
    history.value = history.value.filter((_, i) => i !== index);
  }

  return {
    aesCode,
    customKey,
    customIv,
    history,
    setAesCode,
    setCustomKey,
    setCustomIv,
    rememberCustom,
    pushHistory,
    clearHistory,
    removeHistory,
    removeHistoryAt,
  };
});

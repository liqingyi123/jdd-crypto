<script setup lang="ts">
import { onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { runAes } from "@/services/aes-ops";
import { beautifyText } from "@/utils/text-diff";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

const tip = shallowRef("等待文本选中...");
const busy = shallowRef(false);

let unlistenTip: (() => void) | undefined;
let unlistenSelection: (() => void) | undefined;
let unlistenMode: (() => void) | undefined;

async function handleSelection(cipher: string) {
  if (!cipher?.trim() || busy.value) {
    return;
  }
  busy.value = true;
  try {
    const result = runAes({
      type: "decrypt",
      text: cipher,
      aesCode: "auto",
      customKey: "",
      customIv: "",
    });
    if (result.code !== "ok" || !result.content) {
      tip.value = "解密失败请重新选择文本";
      await invoke("compare_report_fail").catch(() => undefined);
      return;
    }
    const plain = beautifyText(result.content);
    await invoke("compare_report_plain", { text: plain });
  } catch {
    tip.value = "解密失败请重新选择文本";
    await invoke("compare_report_fail").catch(() => undefined);
  } finally {
    busy.value = false;
  }
}

onMounted(async () => {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenTip = await listen<string>("app://compare-tip", (event) => {
      if (typeof event.payload === "string" && event.payload) {
        tip.value = event.payload;
      }
    });
    unlistenSelection = await listen<string>("app://compare-selection", (event) => {
      if (typeof event.payload === "string") {
        void handleSelection(event.payload);
      }
    });
    unlistenMode = await listen<boolean>("app://compare-mode", (event) => {
      if (event.payload) {
        tip.value = "等待文本选中...";
      }
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlistenTip?.();
  unlistenSelection?.();
  unlistenMode?.();
});
</script>

<template>
  <div class="tip-root">
    <div class="tip">{{ tip }}</div>
  </div>
</template>

<style scoped>
.tip-root {
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  padding: 4px;
  background: transparent;
  pointer-events: none;
  user-select: none;
}

.tip {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--bg-elevated) 92%, transparent);
  color: var(--text);
  font-size: 12px;
  font-weight: 600;
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.22);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  box-sizing: border-box;
}
</style>

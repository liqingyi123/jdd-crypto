<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, shallowRef, useTemplateRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { runAes, runAesPreferDecrypt } from "@/services/aes-ops";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

interface BubblePayload {
  text: string;
  mode: string;
}

const loading = shallowRef(false);
const resultText = shallowRef("");
const errorText = shallowRef("");
const visible = shallowRef(false);
const lastMode = shallowRef<"encrypt" | "decrypt">("decrypt");
const copyHint = shallowRef("");
const sourceText = shallowRef("");
const bodyEl = useTemplateRef<HTMLElement>("bodyEl");
let runToken = 0;

let unlisten: (() => void) | undefined;

/** Pretty-print JSON when possible; otherwise keep original text. */
function beautifyResult(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed) {
    return raw;
  }
  if (
    !(trimmed.startsWith("{") || trimmed.startsWith("[") || trimmed.startsWith('"'))
  ) {
    return raw;
  }
  try {
    return JSON.stringify(JSON.parse(trimmed), null, 2);
  } catch {
    return raw;
  }
}

/** True when decrypted payload is a JSON object or array. */
function isJsonPayload(raw: string): boolean {
  const trimmed = raw.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
    return false;
  }
  try {
    const parsed: unknown = JSON.parse(trimmed);
    return parsed !== null && typeof parsed === "object";
  } catch {
    return false;
  }
}

async function scrollBodyToBottom() {
  await nextTick();
  const el = bodyEl.value;
  if (!el) {
    return;
  }
  el.scrollTop = el.scrollHeight;
}

async function closeBubble() {
  visible.value = false;
  resultText.value = "";
  errorText.value = "";
  sourceText.value = "";
  await invoke("hide_crypto_bubble").catch(() => undefined);
}

async function openMain() {
  const mode = lastMode.value;
  const text = sourceText.value;
  await invoke("navigate_main", {
    route: "/",
    mode,
    text,
  }).catch(() => undefined);
  await closeBubble();
}

async function runBubble(payload: BubblePayload) {
  if (!payload?.text) {
    return;
  }
  const silentJson = payload.mode === "silent_json";
  const preferDecrypt = payload.mode === "auto";
  const mode = payload.mode === "encrypt" ? "encrypt" : "decrypt";
  const token = ++runToken;
  lastMode.value = preferDecrypt || silentJson ? "decrypt" : mode;
  sourceText.value = payload.text;
  // Silent path stays invisible until decrypt+JSON succeed.
  visible.value = !silentJson;
  loading.value = !silentJson;
  resultText.value = "";
  errorText.value = "";
  copyHint.value = "";

  try {
    if (silentJson) {
      const result = runAes({
        type: "decrypt",
        text: payload.text,
        aesCode: "auto",
        customKey: "",
        customIv: "",
      });
      if (token !== runToken) {
        return;
      }
      if (result.code !== "ok" || !result.content || !isJsonPayload(result.content)) {
        await invoke("hide_crypto_bubble").catch(() => undefined);
        return;
      }
      lastMode.value = "decrypt";
      resultText.value = beautifyResult(result.content);
      visible.value = true;
      loading.value = false;
      await scrollBodyToBottom();
      return;
    }

    const outcome: {
      mode: "encrypt" | "decrypt";
      result: ReturnType<typeof runAes>;
    } = preferDecrypt
      ? runAesPreferDecrypt(payload.text)
      : {
          mode,
          result: runAes({
            type: mode,
            text: payload.text,
            aesCode: "auto",
            customKey: "",
            customIv: "",
          }),
        };
    const { mode: resolvedMode, result } = outcome;
    if (token !== runToken) {
      return;
    }
    lastMode.value = resolvedMode;
    if (result.code !== "ok" || !result.content) {
      errorText.value = preferDecrypt
        ? "自动加解密失败"
        : resolvedMode === "decrypt"
          ? "自动解密失败"
          : "自动加密失败";
      return;
    }
    resultText.value = beautifyResult(result.content);
  } catch {
    if (token !== runToken) {
      return;
    }
    if (silentJson) {
      await invoke("hide_crypto_bubble").catch(() => undefined);
      return;
    }
    errorText.value = preferDecrypt
      ? "自动加解密失败"
      : mode === "decrypt"
        ? "自动解密失败"
        : "自动加密失败";
  } finally {
    if (token === runToken && !silentJson) {
      loading.value = false;
      if (resultText.value) {
        await scrollBodyToBottom();
      }
    }
  }
}

async function onCopy() {
  if (!resultText.value) {
    return;
  }
  try {
    // 静默写入，避免再次弹出剪贴板询问框
    await invoke("copy_text_silent", { text: resultText.value });
    await closeBubble();
  } catch {
    copyHint.value = "复制失败";
  }
}

async function onDragStart(event: PointerEvent) {
  // Ignore drag starting from interactive controls
  const target = event.target as HTMLElement | null;
  if (target?.closest("button, a, pre, .body")) {
    return;
  }
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().startDragging();
  } catch {
    // browser preview
  }
}

onMounted(async () => {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<BubblePayload>("app://crypto-bubble", (event) => {
      void runBubble(event.payload);
    });
    const pending = await invoke<BubblePayload | null>("get_crypto_bubble_payload").catch(
      () => null,
    );
    if (pending?.text) {
      void runBubble(pending);
    }
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <div v-if="visible" class="bubble-root">
    <div class="bubble" @pointerdown="onDragStart">
      <header class="header">
        <p class="title">{{ lastMode === "encrypt" ? "加密结果" : "解密结果" }}</p>
        <button type="button" class="close" aria-label="关闭" @click.stop="closeBubble">
          ×
        </button>
      </header>
      <div ref="bodyEl" class="body">
        <p v-if="loading" class="muted">处理中…</p>
        <template v-else-if="errorText">
          <p class="error">{{ errorText }}</p>
          <button type="button" class="link" @click.stop="openMain">打开主界面处理</button>
        </template>
        <pre v-else class="result">{{ resultText }}</pre>
      </div>
      <div class="footer">
        <span class="hint">{{ copyHint }}</span>
        <button
          type="button"
          class="copy"
          :disabled="loading || !resultText"
          @click.stop="onCopy"
        >
          复制
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.bubble-root {
  width: 100%;
  height: 100%;
  padding: 10px;
  box-sizing: border-box;
  background: transparent;
  overflow: hidden;
}

.bubble {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 0;
  border-radius: 14px;
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  color: var(--text);
  box-sizing: border-box;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
  overflow: hidden;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 12px 12px 8px 14px;
  cursor: grab;
  user-select: none;
  flex-shrink: 0;
}

.header:active {
  cursor: grabbing;
}

.close {
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--text-muted);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  flex-shrink: 0;
}

.close:hover {
  background: var(--bg-muted);
  color: var(--text);
}

.title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
}

.body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 10px 10px 14px;
  cursor: text;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--text-muted) 45%, transparent) transparent;
}

.body::-webkit-scrollbar {
  width: 8px;
}

.body::-webkit-scrollbar-track {
  background: transparent;
}

.body::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--text-muted) 40%, transparent);
  border-radius: 999px;
  border: 2px solid transparent;
  background-clip: content-box;
}

.body::-webkit-scrollbar-thumb:hover {
  background: color-mix(in srgb, var(--text-muted) 65%, transparent);
  border: 2px solid transparent;
  background-clip: content-box;
}

.body::-webkit-scrollbar-button {
  display: none;
  width: 0;
  height: 0;
}

.muted {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
}

.error {
  margin: 0 0 8px;
  color: var(--danger, #ef4444);
  font-size: 12px;
}

.link {
  border: 0;
  padding: 0;
  background: transparent;
  color: var(--brand);
  font-size: 12px;
  cursor: pointer;
  text-decoration: underline;
}

.result {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
  font-size: 12px;
  line-height: 1.55;
  color: var(--text);
}

.footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 10px 14px 12px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  cursor: grab;
  user-select: none;
}

.footer:active {
  cursor: grabbing;
}

.hint {
  color: var(--text-muted);
  font-size: 12px;
  min-height: 1em;
}

.copy {
  border: 0;
  border-radius: 8px;
  padding: 7px 16px;
  background: var(--brand);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.copy:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>

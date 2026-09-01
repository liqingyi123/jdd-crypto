<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { beautifyText, diffHighlightRight, type DiffSegment } from "@/utils/text-diff";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

interface ComparePayload {
  left: string;
  right: string;
}

const visible = shallowRef(false);
const leftText = shallowRef("");
const rightText = shallowRef("");

const rightSegments = computed<DiffSegment[]>(() =>
  diffHighlightRight(leftText.value, rightText.value),
);

let unlisten: (() => void) | undefined;

async function closeBubble() {
  visible.value = false;
  leftText.value = "";
  rightText.value = "";
  await invoke("hide_compare_bubble").catch(() => undefined);
}

function applyPayload(payload: ComparePayload) {
  if (!payload?.left && !payload?.right) {
    return;
  }
  leftText.value = beautifyText(payload.left ?? "");
  rightText.value = beautifyText(payload.right ?? "");
  visible.value = true;
}

async function onDragStart(event: PointerEvent) {
  const target = event.target as HTMLElement | null;
  if (target?.closest("button, .col-body, pre")) {
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
    unlisten = await listen<ComparePayload>("app://compare-bubble", (event) => {
      applyPayload(event.payload);
    });
    const pending = await invoke<ComparePayload | null>("get_compare_bubble_payload").catch(
      () => null,
    );
    if (pending) {
      applyPayload(pending);
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
        <p class="title">文本对比</p>
        <button type="button" class="close" aria-label="关闭" @click.stop="closeBubble">
          ×
        </button>
      </header>
      <div class="columns">
        <section class="col">
          <h3>前段文本</h3>
          <div class="col-body">
            <pre>{{ leftText }}</pre>
          </div>
        </section>
        <section class="col">
          <h3>后段文本</h3>
          <div class="col-body">
            <pre class="diff-pre"><span
              v-for="(seg, idx) in rightSegments"
              :key="idx"
              :class="['seg', seg.kind]"
            >{{ seg.text }}</span></pre>
          </div>
        </section>
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
  display: flex;
  flex-direction: column;
  height: 100%;
  border-radius: 14px;
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  color: var(--text);
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

.title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
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
}

.close:hover {
  background: var(--bg-muted);
  color: var(--text);
}

.columns {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  padding: 0 12px 12px;
}

.col {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  background: var(--bg);
}

.col h3 {
  margin: 0;
  padding: 8px 10px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.col-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 10px;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--text-muted) 45%, transparent) transparent;
}

.col-body::-webkit-scrollbar {
  width: 8px;
}

.col-body::-webkit-scrollbar-track {
  background: transparent;
}

.col-body::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--text-muted) 40%, transparent);
  border-radius: 999px;
  border: 2px solid transparent;
  background-clip: content-box;
}

.col-body::-webkit-scrollbar-button {
  display: none;
  width: 0;
  height: 0;
}

pre,
.diff-pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
  font-size: 12px;
  line-height: 1.55;
}

.seg.equal {
  color: var(--text);
}

.seg.add,
.seg.change {
  background: color-mix(in srgb, #f59e0b 35%, transparent);
  color: var(--text);
  border-radius: 2px;
}
</style>

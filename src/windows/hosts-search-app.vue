<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  shallowRef,
  useTemplateRef,
  watch,
} from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

interface HostsSearchHit {
  schemeId: string;
  title: string;
  enabled: boolean;
  lines: string[];
}

const SEARCH_WIDTH = 360;
const COMPACT_HEIGHT = 110;
const EXPANDED_HEIGHT = 420;

const query = shallowRef("");
const hits = ref<HostsSearchHit[]>([]);
const searching = shallowRef(false);
const inputRef = useTemplateRef<{ focus: () => void }>("inputRef");

const showResults = computed(() => query.value.trim().length > 0);

let unlistenOpen: UnlistenFn | undefined;
let searchTimer: ReturnType<typeof setTimeout> | undefined;
let searchToken = 0;
let resizeToken = 0;

async function hideWindow() {
  await invoke("hide_hosts_search").catch(() => undefined);
}

async function applyWindowHeight(expanded: boolean) {
  const token = ++resizeToken;
  const height = expanded ? EXPANDED_HEIGHT : COMPACT_HEIGHT;
  try {
    await getCurrentWindow().setSize(new LogicalSize(SEARCH_WIDTH, height));
    if (token !== resizeToken) {
      return;
    }
  } catch {
    // browser preview / ACL denied
  }
}

async function onTitleDragStart(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  try {
    await invoke("hosts_search_begin_drag");
    await getCurrentWindow().startDragging();
  } catch {
    // browser preview
  }
}

async function runSearch(raw: string) {
  const token = ++searchToken;
  const q = raw.trim();
  if (!q) {
    hits.value = [];
    searching.value = false;
    return;
  }
  searching.value = true;
  try {
    const list = await invoke<HostsSearchHit[]>("hosts_search", { query: q });
    if (token !== searchToken) {
      return;
    }
    hits.value = Array.isArray(list) ? list : [];
  } catch {
    if (token !== searchToken) {
      return;
    }
    hits.value = [];
  } finally {
    if (token === searchToken) {
      searching.value = false;
    }
  }
}

function onQueryInput(value: string) {
  query.value = value;
  if (searchTimer !== undefined) {
    clearTimeout(searchTimer);
  }
  searchTimer = setTimeout(() => {
    searchTimer = undefined;
    void runSearch(value);
  }, 200);
}

function resetPanel() {
  query.value = "";
  hits.value = [];
  searching.value = false;
  searchToken += 1;
  if (searchTimer !== undefined) {
    clearTimeout(searchTimer);
    searchTimer = undefined;
  }
  void applyWindowHeight(false);
  void nextTick(() => {
    inputRef.value?.focus();
  });
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    void hideWindow();
  }
}

async function openScheme(schemeId: string, line?: string) {
  await invoke("open_hosts_scheme", {
    schemeId,
    line: line?.trim() || null,
  }).catch(() => undefined);
}

watch(showResults, (expanded) => {
  void applyWindowHeight(expanded);
});

onMounted(async () => {
  resetPanel();
  window.addEventListener("keydown", onKeydown);
  unlistenOpen = await listen("hosts://search-open", () => {
    resetPanel();
  });
});

onUnmounted(() => {
  if (searchTimer !== undefined) {
    clearTimeout(searchTimer);
  }
  window.removeEventListener("keydown", onKeydown);
  void unlistenOpen?.();
});
</script>

<template>
  <div class="search" :class="{ compact: !showResults }">
    <div class="search-head" @pointerdown="onTitleDragStart">Host 检索</div>
    <ElInput
      ref="inputRef"
      :model-value="query"
      clearable
      placeholder="输入域名或 IP"
      @update:model-value="onQueryInput"
    />
    <div v-if="showResults" v-loading="searching" class="search-list">
      <div v-for="hit in hits" :key="hit.schemeId" class="search-hit">
        <div
          class="search-hit-head"
          role="button"
          tabindex="0"
          @click.stop="openScheme(hit.schemeId)"
          @keydown.enter.prevent="openScheme(hit.schemeId)"
        >
          <span class="search-title" :title="hit.title">{{ hit.title }}</span>
          <span class="search-badge" :class="{ on: hit.enabled }">
            {{ hit.enabled ? "已启用" : "未启用" }}
          </span>
        </div>
        <pre
          v-for="(line, index) in hit.lines"
          :key="`${hit.schemeId}-${index}`"
          class="search-line"
          role="button"
          tabindex="0"
          @click.stop="openScheme(hit.schemeId, line)"
          @keydown.enter.prevent="openScheme(hit.schemeId, line)"
        >{{ line }}</pre>
      </div>
      <p v-if="!searching && hits.length === 0" class="search-empty">
        未找到匹配项
      </p>
    </div>
  </div>
</template>

<style scoped>
.search {
  box-sizing: border-box;
  height: 100%;
  padding: 12px;
  border-radius: 10px;
  background: var(--el-bg-color-overlay, var(--el-bg-color));
  border: 1px solid var(--el-border-color-lighter);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow: hidden;
}

.search.compact {
  height: auto;
}

.search-head {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  padding: 2px 4px;
  cursor: grab;
  user-select: none;
}

.search-head:active {
  cursor: grabbing;
}

.search-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.search-hit {
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--el-fill-color-blank, transparent);
  border: 1px solid var(--el-border-color-extra-light);
}

.search-hit-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
  padding: 4px 6px;
  margin-left: -6px;
  margin-right: -6px;
  cursor: pointer;
  border-radius: 6px;
}

.search-hit-head:hover,
.search-hit-head:focus-visible {
  outline: none;
  background: color-mix(in srgb, var(--el-color-primary) 18%, transparent);
  color: var(--el-text-color-primary);
}

.search-title {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-badge {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--el-text-color-secondary);
}

.search-badge.on {
  color: var(--el-color-success);
}

.search-line {
  margin: 0 0 4px;
  padding: 4px 6px;
  border-radius: 4px;
  background: var(--el-fill-color-light);
  font-size: 11px;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
  color: var(--el-text-color-regular);
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease;
}

.search-line:hover,
.search-line:focus-visible {
  outline: none;
  background: color-mix(
    in srgb,
    var(--el-color-primary) 28%,
    var(--el-fill-color-light)
  );
  color: var(--el-text-color-primary);
  box-shadow: inset 0 0 0 1px var(--el-color-primary-light-5);
}

.search-line:last-child {
  margin-bottom: 0;
}

.search-empty {
  margin: 16px 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  text-align: center;
}
</style>

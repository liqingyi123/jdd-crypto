<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

interface DisplayBrightness {
  id: string;
  name: string;
  brightness: number;
  min: number;
  max: number;
  supported: boolean;
}

const displays = ref<DisplayBrightness[]>([]);
const loading = shallowRef(false);
const savingId = shallowRef<string | null>(null);

let unlistenOpen: UnlistenFn | undefined;
let unlistenBlur: (() => void) | undefined;
let blurArmed = false;
let armTimer: ReturnType<typeof setTimeout> | undefined;
let loadToken = 0;
const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();

async function hideWindow() {
  await invoke("hide_brightness_bubble").catch(() => undefined);
}

function showToast(message: string) {
  void invoke("show_overlay_toast", { message }).catch(() => undefined);
}

function clearDebounceTimers() {
  for (const timer of debounceTimers.values()) {
    clearTimeout(timer);
  }
  debounceTimers.clear();
}

async function loadDisplays(preset?: DisplayBrightness[]) {
  const token = ++loadToken;
  loading.value = true;
  savingId.value = null;
  try {
    const list =
      preset ??
      (await invoke<DisplayBrightness[]>("list_display_brightness"));
    if (token !== loadToken) {
      return;
    }
    displays.value = Array.isArray(list) ? list : [];
  } catch (err) {
    if (token !== loadToken) {
      return;
    }
    showToast(`加载亮度失败：${String(err)}`);
    await hideWindow();
  } finally {
    if (token === loadToken) {
      loading.value = false;
    }
  }
}

function percentOf(display: DisplayBrightness): number {
  const span = Math.max(1, display.max - display.min);
  return Math.round(((display.brightness - display.min) / span) * 100);
}

function valueFromPercent(display: DisplayBrightness, percent: number): number {
  const span = Math.max(1, display.max - display.min);
  return Math.round(display.min + (percent / 100) * span);
}

function onSliderInput(display: DisplayBrightness, percent: number | number[]) {
  const p = Array.isArray(percent) ? Number(percent[0]) : Number(percent);
  if (!Number.isFinite(p) || !display.supported) {
    return;
  }
  const next = valueFromPercent(display, p);
  display.brightness = next;
  const existing = debounceTimers.get(display.id);
  if (existing !== undefined) {
    clearTimeout(existing);
  }
  debounceTimers.set(
    display.id,
    setTimeout(() => {
      debounceTimers.delete(display.id);
      void persistBrightness(display.id, next);
    }, 80),
  );
}

async function persistBrightness(id: string, value: number) {
  savingId.value = id;
  try {
    await invoke("set_display_brightness", { id, value });
  } catch (err) {
    showToast(`调节失败：${String(err)}`);
    await loadDisplays();
  } finally {
    if (savingId.value === id) {
      savingId.value = null;
    }
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    void hideWindow();
  }
}

function clearArmTimer() {
  if (armTimer !== undefined) {
    clearTimeout(armTimer);
    armTimer = undefined;
  }
}

function armBlurDismiss() {
  clearArmTimer();
  blurArmed = false;
  armTimer = setTimeout(() => {
    blurArmed = true;
  }, 280);
}

onMounted(async () => {
  await loadDisplays();
  armBlurDismiss();
  window.addEventListener("keydown", onKeydown);
  const win = getCurrentWindow();
  unlistenBlur = await win.onFocusChanged(({ payload: focused }) => {
    if (!focused && blurArmed && !savingId.value) {
      void hideWindow();
    }
  });
  unlistenOpen = await listen<DisplayBrightness[]>("brightness://quick-open", (event) => {
    loading.value = false;
    savingId.value = null;
    void loadDisplays(event.payload);
    armBlurDismiss();
  });
});

onUnmounted(() => {
  clearArmTimer();
  clearDebounceTimers();
  window.removeEventListener("keydown", onKeydown);
  unlistenBlur?.();
  void unlistenOpen?.();
});
</script>

<template>
  <div v-loading="loading" class="quick">
    <div class="quick-head">拖动滑块调节对应屏幕亮度</div>
    <div class="quick-list">
      <div v-for="display in displays" :key="display.id" class="quick-item">
        <div class="quick-meta">
          <div class="quick-title" :title="display.name">{{ display.name }}</div>
          <div class="quick-value">
            {{ display.supported ? `${percentOf(display)}%` : "不支持" }}
          </div>
        </div>
        <ElSlider
          :model-value="percentOf(display)"
          :min="0"
          :max="100"
          :step="1"
          :disabled="!display.supported || savingId === display.id"
          :show-tooltip="false"
          @input="(value: number | number[]) => onSliderInput(display, value)"
        />
      </div>
      <p v-if="!loading && displays.length === 0" class="quick-empty">
        未检测到显示器
      </p>
    </div>
  </div>
</template>

<style scoped>
.quick {
  box-sizing: border-box;
  height: 100%;
  padding: 10px;
  border-radius: 10px;
  background: var(--el-bg-color-overlay, var(--el-bg-color));
  border: 1px solid var(--el-border-color-lighter);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow: hidden;
}

.quick-head {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  padding: 2px 4px;
  flex-shrink: 0;
  line-height: 1.35;
}

.quick-list {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.quick-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 10px;
  border-radius: 8px;
}

.quick-item:hover {
  background: var(--el-fill-color-light);
}

.quick-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}

.quick-title {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.quick-value {
  flex-shrink: 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  font-variant-numeric: tabular-nums;
}

.quick-empty {
  margin: 16px 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  text-align: center;
}
</style>

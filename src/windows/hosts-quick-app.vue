<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ElMessage } from "element-plus";
import { useSystemTheme } from "@/composables/use-system-theme";

useSystemTheme();

interface HostsScheme {
  id: string;
  title: string;
  enabled: boolean;
  nature: string;
  type?: string;
  readonly?: boolean;
}

const schemes = ref<HostsScheme[]>([]);
const loading = shallowRef(false);
const togglingId = shallowRef<string | null>(null);

let unlistenOpen: UnlistenFn | undefined;
let unlistenBlur: (() => void) | undefined;
let blurArmed = false;
let armTimer: ReturnType<typeof setTimeout> | undefined;

async function hideWindow() {
  await invoke("hide_hosts_quick").catch(() => undefined);
}

async function loadSchemes() {
  loading.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_list");
    schemes.value = Array.isArray(list) ? list : [];
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    loading.value = false;
  }
}

async function toggleEnabled(scheme: HostsScheme, enabled: boolean) {
  togglingId.value = scheme.id;
  try {
    const list = await invoke<HostsScheme[]>("hosts_set_enabled", {
      id: scheme.id,
      enabled,
    });
    schemes.value = list;
    ElMessage.success(
      enabled ? `已启用「${scheme.title}」` : `已关闭「${scheme.title}」`,
    );
    await hideWindow();
  } catch (err) {
    ElMessage.error(`切换失败：${String(err)}`);
    await loadSchemes();
  } finally {
    togglingId.value = null;
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
  await loadSchemes();
  armBlurDismiss();
  window.addEventListener("keydown", onKeydown);
  const win = getCurrentWindow();
  unlistenBlur = await win.onFocusChanged(({ payload: focused }) => {
    if (!focused && blurArmed) {
      void hideWindow();
    }
  });
  unlistenOpen = await listen("hosts://quick-open", () => {
    void loadSchemes();
    armBlurDismiss();
  });
});

onUnmounted(() => {
  clearArmTimer();
  window.removeEventListener("keydown", onKeydown);
  unlistenBlur?.();
  void unlistenOpen?.();
});
</script>

<template>
  <div v-loading="loading" class="quick">
    <div class="quick-head">Host 快速切换</div>
    <div class="quick-list">
      <div v-for="scheme in schemes" :key="scheme.id" class="quick-item">
        <div class="quick-title" :title="scheme.title">{{ scheme.title }}</div>
        <ElSwitch
          :model-value="scheme.enabled"
          size="small"
          :disabled="togglingId === scheme.id"
          @change="(value) => toggleEnabled(scheme, Boolean(value))"
        />
      </div>
      <p v-if="!loading && schemes.length === 0" class="quick-empty">
        暂无方案
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
}

.quick-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.quick-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 8px;
}

.quick-item:hover {
  background: var(--el-fill-color-light);
}

.quick-title {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.quick-empty {
  margin: 16px 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  text-align: center;
}
</style>

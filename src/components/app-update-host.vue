<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import AppUpdateDialog from "@/components/app-update-dialog.vue";
import {
  applyUpdateResult,
  type UpdateCheckResult,
} from "@/composables/use-app-update";
import zhCn from "element-plus/es/locale/lang/zh-cn";

let unlistenUpdate: (() => void) | undefined;

onMounted(async () => {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenUpdate = await listen<UpdateCheckResult>(
      "app://update-available",
      (event) => {
        applyUpdateResult(event.payload);
      },
    );
  } catch {
    // browser preview
  }

  try {
    const pending = await invoke<UpdateCheckResult | null>("take_pending_app_update");
    if (pending) {
      applyUpdateResult(pending);
    }
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlistenUpdate?.();
});
</script>

<template>
  <ElConfigProvider :locale="zhCn">
    <AppUpdateDialog />
  </ElConfigProvider>
</template>

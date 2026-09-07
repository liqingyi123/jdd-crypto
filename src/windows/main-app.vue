<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useSystemTheme } from "@/composables/use-system-theme";
import { useClipboardPrompt } from "@/composables/use-clipboard-prompt";
import { useAppStore } from "@/stores/app";
import CryptoHome from "@/views/crypto-home.vue";
import zhCn from "element-plus/es/locale/lang/zh-cn";

useSystemTheme();
useClipboardPrompt();

const appStore = useAppStore();

let unlistenPayload: (() => void) | undefined;

onMounted(async () => {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenPayload = await listen<{ text: string; mode: string }>(
      "app://crypto-payload",
      (event) => {
        appStore.setPendingCrypto(event.payload);
      },
    );
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlistenPayload?.();
});
</script>

<template>
  <ElConfigProvider :locale="zhCn">
    <div class="shell">
      <header class="header">
        <h1 class="title">加解密</h1>
      </header>
      <section class="content">
        <CryptoHome />
      </section>
    </div>
  </ElConfigProvider>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.header {
  padding: 18px 24px 8px;
}

.title {
  margin: 0;
  font-size: 20px;
}

.content {
  flex: 1;
  min-height: 0;
  padding: 8px 24px 24px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.content :deep(.page) {
  flex: 1;
  min-height: 0;
}
</style>

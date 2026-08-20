<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { ElMessage } from "element-plus";
import PluginTypeCard from "@/components/plugin-type-card.vue";
import {
  PLUGIN_KIND_META,
  type PluginKind,
} from "@/constants/plugin-slots";
import { usePluginSlots } from "@/composables/use-plugin-slots";

const kinds: PluginKind[] = ["editor-theme", "crypto-preset"];

const {
  loading,
  importingKind,
  resettingKind,
  refresh,
  slotOf,
  toggleSlot,
  resetSlot,
  importSlot,
} = usePluginSlots();

const fileInputs = ref<Record<PluginKind, HTMLInputElement | null>>({
  "editor-theme": null,
  "crypto-preset": null,
});

let unlistenSlots: (() => void) | undefined;

onMounted(async () => {
  await refresh();
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenSlots = await listen("app://plugin-slots", () => {
      void refresh();
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  unlistenSlots?.();
});

function triggerImport(kind: PluginKind) {
  if (PLUGIN_KIND_META[kind].comingSoon) {
    ElMessage.warning("开发中，敬请期待");
    return;
  }
  fileInputs.value[kind]?.click();
}

async function onFileSelected(kind: PluginKind, event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) {
    return;
  }
  await importSlot(kind, file);
}
</script>

<template>
  <div v-loading="loading" class="page">
    <p class="lead">每种插件同时只能导入并应用一个；导入新文件会覆盖同类型旧插件。</p>

    <PluginTypeCard
      v-for="kind in kinds"
      :key="kind"
      :title="PLUGIN_KIND_META[kind].title"
      :description="PLUGIN_KIND_META[kind].description"
      :slot="slotOf(kind)"
      :coming-soon="PLUGIN_KIND_META[kind].comingSoon"
      :importing="importingKind === kind"
      :resetting="resettingKind === kind"
      @toggle="(enabled) => toggleSlot(kind, enabled)"
      @import="triggerImport(kind)"
      @reset="resetSlot(kind)"
    />

    <input
      v-for="kind in kinds"
      :key="`${kind}-file`"
      :ref="(el) => { fileInputs[kind] = el as HTMLInputElement | null }"
      type="file"
      accept=".json,application/json"
      class="hidden-input"
      @change="(event) => onFileSelected(kind, event)"
    />
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.lead {
  margin: 0;
  color: var(--text-muted);
  font-size: 13px;
}

.hidden-input {
  display: none;
}
</style>

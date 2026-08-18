<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { storeToRefs } from "pinia";
import { usePluginsStore } from "@/stores/plugins";
import { loadPlugin } from "@/plugins-runtime/loader";
import type { PluginManifest } from "@/plugins-runtime/types";

const pluginsStore = usePluginsStore();
const { manifests, cryptoOptions, editors, overlayEffects } = storeToRefs(pluginsStore);
const errorMessage = ref("");

onMounted(async () => {
  await refresh();
});

async function refresh() {
  errorMessage.value = "";
  try {
    const list = await invoke<PluginManifest[]>("list_plugins");
    pluginsStore.setManifests(list);
    for (const manifest of list) {
      await loadPlugin(manifest, {
        registerCryptoOption: pluginsStore.registerCryptoOption,
        registerEditor: pluginsStore.registerEditor,
        registerOverlayEffect: pluginsStore.registerOverlayEffect,
      });
    }
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}
</script>

<template>
  <div class="page">
    <p>插件以 `plugin.json` 清单加载。当前只扫描并注册贡献点，不执行任意原生库。</p>
    <p v-if="errorMessage" class="error">{{ errorMessage }}</p>
    <div v-if="manifests.length" class="list">
      <ElCard v-for="item in manifests" :key="item.id" shadow="never">
        <div class="card-head">
          <strong>{{ item.name }}</strong>
          <span>{{ item.id }} · v{{ item.version }}</span>
        </div>
        <p>权限：{{ item.permissions.join(", ") || "无" }}</p>
        <p>目录：{{ item.dir }}</p>
      </ElCard>
    </div>
    <ElEmpty v-else description="未发现插件。可把插件放到 src-tauri/plugins/<id>/" />
    <div class="caps">
      <p>已注册编辑器：{{ editors.map((item) => item.label).join(", ") || "无" }}</p>
      <p>已注册特效：{{ overlayEffects.map((item) => item.label).join(", ") || "无" }}</p>
      <p>已注册算法选项：{{ cryptoOptions.map((item) => item.label).join(", ") || "无" }}</p>
    </div>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.card-head {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

p,
span {
  color: var(--text-muted);
  margin: 6px 0 0;
}

.error {
  color: var(--danger);
}
</style>

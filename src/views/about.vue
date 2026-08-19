<script setup lang="ts">
import { onMounted, ref } from "vue";

const appIcon = "/app-icon.png";
const version = ref("0.1.0");
const platform = ref("desktop");

onMounted(async () => {
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    version.value = await getVersion();
  } catch {
    // browser preview
  }
  try {
    const { platform: readPlatform } = await import("@tauri-apps/plugin-os");
    platform.value = readPlatform();
  } catch {
    platform.value = navigator.platform;
  }
});
</script>

<template>
  <div class="page">
    <img class="logo" :src="appIcon" alt="多多解密" />
    <h2>多多解密</h2>
    <p>奖多多内部专用跨平台加解密工具</p>
    <dl>
      <div>
        <dt>版本</dt>
        <dd>{{ version }}</dd>
      </div>
      <div>
        <dt>平台</dt>
        <dd>{{ platform }}</dd>
      </div>
      <div>
        <dt>作者</dt>
        <dd>二六得八（二楼得爬）</dd>
      </div>
      <div>
        <dt>联系方式</dt>
        <dd>QQ：1787750205 | wx：qinghe6971</dd>
      </div>
    </dl>
  </div>
</template>

<style scoped>
.page {
  max-width: 520px;
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-elevated);
}

.logo {
  width: 56px;
  height: 56px;
  border-radius: 10px;
  object-fit: cover;
  image-rendering: pixelated;
  margin-bottom: 12px;
}

h2 {
  margin: 0 0 8px;
}

p,
dd {
  color: var(--text-muted);
}

dl {
  display: grid;
  gap: 8px;
}

dl > div {
  display: grid;
  grid-template-columns: 80px 1fr;
}

dt {
  font-weight: 600;
}

dd {
  margin: 0;
}
</style>

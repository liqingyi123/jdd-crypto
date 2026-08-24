<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useAppUpdate } from "@/composables/use-app-update";

const appIcon = "/app-icon.png";
const version = ref("0.1.0");
const platform = ref("desktop");
const { checking, checkUpdate } = useAppUpdate();

const roadmap = [
  "转 KV 功能支持密文、明文两种文本格式",
  "网络代理功能，实时捕获任意应用的网络请求",
  "插件管理功能支持编辑器主题、加解密预设",
  "彩虹屁 + 节日祝福语功能",
  "将结果自动复制到剪贴板"
];
const callmap = [
  { type: 'QQ', value: '1787750205' },
  { type: '微信', value: 'qinghe6971' }
];

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

function onCheckUpdate() {
  void checkUpdate(true);
}
</script>

<template>
  <div class="page">
    <img class="logo" :src="appIcon" alt="多多解密" />
    <h2>多多解密</h2>
    <p class="lead">奖多多内部专用跨平台桌面端加解密工具</p>

    <dl class="meta">
      <div>
        <dt>版本</dt>
        <dd class="version-row">
          <span>{{ version }}</span>
          <ElButton link type="primary" :loading="checking" @click="onCheckUpdate">
            检查更新
          </ElButton>
        </dd>
      </div>
      <div>
        <dt>平台</dt>
        <dd>{{ platform }}</dd>
      </div>
      <div>
        <dt>作者</dt>
        <dd>李青逸（二六得八 / 二楼得爬）</dd>
      </div>
    </dl>
    <section class="roadmap">
      <h3>联系方式</h3>
      <ol>
        <li v-for="item in callmap" :key="item.type">
          <span class="index">{{ item.type }}</span>
          <span class="text">{{ item.value }}</span>
        </li>
      </ol>
    </section>
    <section class="roadmap">
      <h3>规划中</h3>
      <ol>
        <li v-for="(item, index) in roadmap" :key="item">
          <span class="index">{{ index + 1 }}</span>
          <span class="text">{{ item }}</span>
        </li>
      </ol>
    </section>
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

.lead {
  margin: 0 0 16px;
  color: var(--text-muted);
}

.meta {
  display: grid;
  gap: 8px;
  margin: 0 0 18px;
}

.meta > div {
  display: grid;
  grid-template-columns: 80px 1fr;
  gap: 8px;
}

dt {
  font-weight: 600;
}

dd {
  margin: 0;
  color: var(--text-muted);
}

.version-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.roadmap h3 {
  margin: 0 0 10px;
  font-size: 14px;
  font-weight: 600;
}

.roadmap ol {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
}

.roadmap li {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-muted);
}

.index {
  flex: 0 0 auto;
  width: 22px;
  height: 22px;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  color: var(--brand);
  background: var(--brand-soft);
  white-space: nowrap;
}

.text {
  flex: 1;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text);
}
</style>

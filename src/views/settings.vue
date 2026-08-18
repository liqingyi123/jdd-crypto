<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { storeToRefs } from "pinia";
import { useThemeStore, type ThemePreference } from "@/stores/theme";
import { useClipboardStore } from "@/stores/clipboard";

const themeStore = useThemeStore();
const clipboardStore = useClipboardStore();
const { preference } = storeToRefs(themeStore);
const { watchEnabled } = storeToRefs(clipboardStore);

const themeOptions: Array<{ value: ThemePreference; label: string }> = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
];

const badgeSize = ref(96);
const badgeSizeOptions: Array<{ value: number; label: string }> = [
  { value: 96, label: "大" },
  { value: 68, label: "中" },
  { value: 38, label: "小" },
];

onMounted(async () => {
  try {
    const enabled = await invoke<boolean>("get_clipboard_watch");
    clipboardStore.setWatchEnabled(enabled);
  } catch {
    // ignore
  }
  try {
    badgeSize.value = await invoke<number>("get_badge_size");
  } catch {
    // browser preview
  }
});

async function onWatchChange(event: Event) {
  const enabled = (event.target as HTMLInputElement).checked;
  clipboardStore.setWatchEnabled(enabled);
  await invoke("set_clipboard_watch", { enabled }).catch(() => undefined);
}

async function onBadgeSizeChange(size: number) {
  badgeSize.value = size;
  await invoke("set_badge_size_pref", { size }).catch(() => undefined);
}
</script>

<template>
  <div class="page">
    <section>
      <h2>外观</h2>
      <div class="row">
        <label v-for="item in themeOptions" :key="item.value">
          <input
            type="radio"
            name="theme"
            :value="item.value"
            :checked="preference === item.value"
            @change="themeStore.setPreference(item.value)"
          />
          {{ item.label }}
        </label>
      </div>
      <h3>角标大小</h3>
      <div class="row">
        <label v-for="item in badgeSizeOptions" :key="item.value">
          <input
            type="radio"
            name="badge-size"
            :value="item.value"
            :checked="badgeSize === item.value"
            @change="onBadgeSizeChange(item.value)"
          />
          {{ item.label }}
        </label>
      </div>
    </section>
    <section>
      <h2>剪贴板</h2>
      <label class="row">
        <input type="checkbox" :checked="watchEnabled" @change="onWatchChange" />
        自动识别剪贴板并提示加解密
      </label>
      <p>关闭后不再轮询剪贴板，避免打扰与隐私风险。</p>
    </section>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

section {
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-elevated);
}

h2 {
  margin: 0 0 12px;
  font-size: 16px;
}

h3 {
  margin: 16px 0 12px;
  font-size: 14px;
  font-weight: 600;
}

.row {
  display: flex;
  gap: 16px;
  align-items: center;
}

p {
  margin: 8px 0 0;
  color: var(--text-muted);
}
</style>

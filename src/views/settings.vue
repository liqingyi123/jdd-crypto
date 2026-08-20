<script setup lang="ts">
import { onMounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage } from "element-plus";
import { storeToRefs } from "pinia";
import { useThemeStore, type ThemePreference } from "@/stores/theme";
import { useClipboardStore } from "@/stores/clipboard";
import { useShortcutRecorder } from "@/composables/use-shortcut-recorder";
import { DEFAULT_BADGE_SIZE } from "@/constants/badge";
import {
  DEFAULT_MOUSE_TRAIL_PREF,
  normalizeMouseTrailEffect,
  type MouseTrailEffect,
  type MouseTrailPref,
} from "@/effects/mouse-trail-types";

const themeStore = useThemeStore();
const clipboardStore = useClipboardStore();
const { preference } = storeToRefs(themeStore);
const { watchEnabled } = storeToRefs(clipboardStore);
const {
  recording,
  errorMessage,
  display,
  previewDisplay,
  buttonRef,
  startRecording,
  cancelRecording,
  onRecordKey,
  loadShortcut,
} = useShortcutRecorder();

const themeOptions: Array<{ value: ThemePreference; label: string }> = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
];

const badgeSize = shallowRef(DEFAULT_BADGE_SIZE);
const followPref = shallowRef(true);
const trailEnabled = shallowRef(DEFAULT_MOUSE_TRAIL_PREF.enabled);
const trailEffect = shallowRef<MouseTrailEffect>(DEFAULT_MOUSE_TRAIL_PREF.effect);
const trailResetting = shallowRef(false);

const badgeSizeOptions: Array<{ value: number; label: string }> = [
  { value: 96, label: "大" },
  { value: DEFAULT_BADGE_SIZE, label: "中" },
  { value: 38, label: "小" },
];

const trailEffectOptions: Array<{ value: MouseTrailEffect; label: string }> = [
  { value: "ribbon", label: "躁动线条" },
  { value: "meteor", label: "绚丽流星" },
  { value: "graffiti", label: "街头涂鸦" },
  { value: "dots", label: "连线点阵" },
];

function applyTrailPref(pref: MouseTrailPref) {
  trailEnabled.value = pref.enabled;
  trailEffect.value = normalizeMouseTrailEffect(pref.effect);
}

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
  try {
    followPref.value = await invoke<boolean>("get_mouse_follow_pref");
  } catch {
    // browser preview
  }
  try {
    const pref = await invoke<MouseTrailPref>("get_mouse_trail_pref");
    applyTrailPref(pref);
  } catch {
    applyTrailPref(DEFAULT_MOUSE_TRAIL_PREF);
  }
  await loadShortcut();
});

async function onWatchChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  clipboardStore.setWatchEnabled(enabled);
  await invoke("set_clipboard_watch", { enabled }).catch(() => undefined);
}

async function onBadgeSizeChange(value: string | number | boolean | undefined) {
  const size = Number(value);
  if (!Number.isFinite(size)) {
    return;
  }
  badgeSize.value = size;
  await invoke("set_badge_size_pref", { size }).catch(() => undefined);
}

async function onFollowPrefChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  followPref.value = enabled;
  if (!enabled && recording.value) {
    await cancelRecording();
  }
  await invoke("set_mouse_follow_pref", { enabled }).catch(() => undefined);
}

async function onTrailEnabledChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  trailEnabled.value = enabled;
  try {
    const pref = await invoke<MouseTrailPref>("set_mouse_trail_enabled", { enabled });
    applyTrailPref(pref);
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
    trailEnabled.value = !enabled;
  }
}

async function onTrailEffectChange(value: string | number | boolean | undefined) {
  if (
    value !== "ribbon" &&
    value !== "meteor" &&
    value !== "graffiti" &&
    value !== "dots"
  ) {
    return;
  }
  trailEffect.value = value;
  try {
    const pref = await invoke<MouseTrailPref>("set_mouse_trail_effect", { effect: value });
    applyTrailPref(pref);
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onTrailReset() {
  trailResetting.value = true;
  try {
    const pref = await invoke<MouseTrailPref>("reset_mouse_trail_pref");
    applyTrailPref(pref);
    ElMessage.success("已恢复默认");
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  } finally {
    trailResetting.value = false;
  }
}

function onThemeChange(value: string | number | boolean | undefined) {
  if (value === "system" || value === "light" || value === "dark") {
    themeStore.setPreference(value);
    void invoke("set_theme_pref", { preference: value }).catch(() => undefined);
  }
}
</script>

<template>
  <div class="page">
    <section>
      <h2>外观</h2>
      <ElRadioGroup :model-value="preference" @change="onThemeChange">
        <ElRadio
          v-for="item in themeOptions"
          :key="item.value"
          :value="item.value"
        >
          {{ item.label }}
        </ElRadio>
      </ElRadioGroup>
      <h3>角标大小</h3>
      <ElRadioGroup :model-value="badgeSize" @change="onBadgeSizeChange">
        <ElRadio
          v-for="item in badgeSizeOptions"
          :key="item.value"
          :value="item.value"
        >
          {{ item.label }}
        </ElRadio>
      </ElRadioGroup>
    </section>
    <section>
      <div class="section-head">
        <h2>剪贴板</h2>
        <label class="row">
          <ElSwitch :model-value="watchEnabled" @change="onWatchChange" />
        </label>
      </div>
      <p>
        自动识别剪贴板并提示加解密，关闭后不再轮询剪贴板，避免打扰与隐私风险。</p>
    </section>
    <section>
      <div class="section-head">
        <h2>鼠标轨迹特效</h2>
        <label class="row">
          <ElSwitch :model-value="trailEnabled" @change="onTrailEnabledChange" />
        </label>
      </div>
      <p>我的鼠标指针哪去啦？？！！<br />在所有显示器工作区跟随鼠标绘制炫酷好玩的拖尾特效以帮助更好的寻找鼠标位置。</p>
      <ElRadioGroup
        :model-value="trailEffect"
        :disabled="!trailEnabled"
        @change="onTrailEffectChange"
      >
        <ElRadio
          v-for="item in trailEffectOptions"
          :key="item.value"
          :value="item.value"
        >
          {{ item.label }}
        </ElRadio>
      </ElRadioGroup>
      <div class="actions">
        <ElButton :loading="trailResetting" @click="onTrailReset">恢复默认</ElButton>
      </div>
    </section>
    <section>
      <div class="section-head">
        <h2>鼠标跟随</h2>
        <label class="row">
          <ElSwitch :model-value="followPref" @change="onFollowPrefChange" />
        </label>
      </div>
      <p>按下快捷键开启后会跟随鼠标移动并自动等待鼠标下次的文本选中，按下鼠标选中文本后松开会自动打开加解密页面并代入选中的文本。</p>
      <div class="row">
        <span>快捷键</span>
        <div ref="buttonRef">
          <ElButton
            :type="recording ? 'primary' : 'default'"
            :disabled="!followPref"
            @click="startRecording"
            @keydown="onRecordKey"
          >
            {{ recording ? previewDisplay : display }}
          </ElButton>
        </div>
      </div>
      <p v-if="errorMessage" class="error">{{ errorMessage }}</p>
      <p>最多 4 个键，需包含修饰键。点击按钮后按下新组合，Esc 或点击其他区域取消。</p>
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

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.section-head h2 {
  margin: 0;
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

.actions {
  margin-top: 12px;
}

p {
  margin: 8px 0 0;
  color: var(--text-muted);
}

.error {
  color: var(--danger, #c0392b);
}
</style>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ElMessage } from "element-plus";
import { storeToRefs } from "pinia";
import { useThemeStore, type ThemePreference } from "@/stores/theme";
import { useClipboardStore } from "@/stores/clipboard";
import { useShortcutRecorder } from "@/composables/use-shortcut-recorder";
import { BADGE_HIDDEN_SIZE, DEFAULT_BADGE_SIZE } from "@/constants/badge";
import {
  DEFAULT_MOUSE_TRAIL_COLORS,
  DEFAULT_MOUSE_TRAIL_PREF,
  isColorableTrailEffect,
  normalizeMouseTrailColors,
  normalizeMouseTrailEffect,
  type ColorableTrailEffect,
  type MouseTrailColors,
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

const {
  recording: compareRecording,
  errorMessage: compareErrorMessage,
  display: compareDisplay,
  previewDisplay: comparePreviewDisplay,
  buttonRef: compareButtonRef,
  startRecording: startCompareRecording,
  cancelRecording: cancelCompareRecording,
  onRecordKey: onCompareRecordKey,
  loadShortcut: loadCompareShortcut,
} = useShortcutRecorder({
  getCommand: "get_compare_mode_shortcut",
  setCommand: "set_compare_mode_shortcut",
  defaultShortcut: "Ctrl+Shift+D",
});

const themeOptions: Array<{ value: ThemePreference; label: string }> = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
];

const badgeSize = shallowRef(DEFAULT_BADGE_SIZE);
const followPref = shallowRef(true);
const comparePref = shallowRef(true);
const autostartEnabled = shallowRef(false);
const trailEnabled = shallowRef(DEFAULT_MOUSE_TRAIL_PREF.enabled);
const trailEffect = shallowRef<MouseTrailEffect>(DEFAULT_MOUSE_TRAIL_PREF.effect);
const trailColors = shallowRef<MouseTrailColors>({ ...DEFAULT_MOUSE_TRAIL_COLORS });
const trailColorsResetting = shallowRef(false);

const activeColorEffect = computed((): ColorableTrailEffect | null => {
  const effect = trailEffect.value;
  return isColorableTrailEffect(effect) ? effect : null;
});

const showTrailColor = computed(
  () => trailEnabled.value && activeColorEffect.value !== null,
);

const badgeSizeOptions: Array<{ value: number; label: string }> = [
  { value: 96, label: "大" },
  { value: DEFAULT_BADGE_SIZE, label: "中" },
  { value: 38, label: "小" },
  { value: BADGE_HIDDEN_SIZE, label: "隐藏" },
];

const trailEffectOptions: Array<{ value: MouseTrailEffect; label: string; shortcutKey: string }> = [
  { value: "ribbon", label: "躁动线条", shortcutKey: "1" },
  { value: "meteor", label: "星痕漫衍", shortcutKey: "2" },
  { value: "graffiti", label: "街头涂鸦", shortcutKey: "3" },
  { value: "dots", label: "浮络牵光", shortcutKey: "4" },
  { value: "heart", label: "绮心逐迹", shortcutKey: "5" },
  { value: "ripple", label: "沧涟曳逝", shortcutKey: "6" },
];

let unlistenTrailPref: UnlistenFn | null = null;
function applyTrailPref(pref: MouseTrailPref) {
  trailEnabled.value = pref.enabled;
  trailEffect.value = normalizeMouseTrailEffect(pref.effect);
  trailColors.value = normalizeMouseTrailColors(pref.colors);
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
    comparePref.value = await invoke<boolean>("get_compare_mode_pref");
  } catch {
    // browser preview
  }
  try {
    autostartEnabled.value = await invoke<boolean>("get_autostart_pref");
  } catch {
    autostartEnabled.value = false;
  }
  try {
    const pref = await invoke<MouseTrailPref>("get_mouse_trail_pref");
    applyTrailPref(pref);
  } catch {
    applyTrailPref(DEFAULT_MOUSE_TRAIL_PREF);
  }
  await loadShortcut();
  await loadCompareShortcut();
  try {
    unlistenTrailPref = await listen<MouseTrailPref>("app://mouse-trail-pref", (event) => {
      applyTrailPref(event.payload);
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  void unlistenTrailPref?.();
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

async function onComparePrefChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  comparePref.value = enabled;
  if (!enabled && compareRecording.value) {
    await cancelCompareRecording();
  }
  await invoke("set_compare_mode_pref", { enabled }).catch(() => undefined);
}

async function onAutostartChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  const previous = autostartEnabled.value;
  autostartEnabled.value = enabled;
  try {
    await invoke<boolean>("set_autostart_pref", { enabled });
  } catch (error) {
    autostartEnabled.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
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
    value !== "dots" &&
    value !== "heart" &&
    value !== "ripple"
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

async function onTrailColorChange(color: string | null) {
  const effect = activeColorEffect.value;
  if (!color || !effect) {
    return;
  }
  try {
    const pref = await invoke<MouseTrailPref>("set_mouse_trail_color", { effect, color });
    applyTrailPref(pref);
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onTrailColorReset() {
  const effect = activeColorEffect.value;
  if (!effect) {
    return;
  }
  trailColorsResetting.value = true;
  try {
    const pref = await invoke<MouseTrailPref>("reset_mouse_trail_colors", { effect });
    applyTrailPref(pref);
    ElMessage.success("已恢复默认颜色");
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  } finally {
    trailColorsResetting.value = false;
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
        <h2>开机自启动</h2>
        <label class="row">
          <ElSwitch :model-value="autostartEnabled" @change="onAutostartChange" />
        </label>
      </div>
      <p>开启后登录系统时自动启动多多解密；默认关闭。</p>
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
      <p class="trail-shortcut-hint">
        快捷键：按住 Ctrl，依次按下 T 与数字键 1–6（Ctrl+T+数字）切换特效；松开 Ctrl 后需重新按下 T。
      </p>
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
          <span class="trail-key">Ctrl+T+{{ item.shortcutKey }}</span>
        </ElRadio>
      </ElRadioGroup>
      <template v-if="showTrailColor && activeColorEffect">
        <h3>特效颜色</h3>
        <div class="row trail-color-row">
          <ElColorPicker
            :model-value="trailColors[activeColorEffect]"
            @change="onTrailColorChange"
          />
          <ElButton :loading="trailColorsResetting" @click="onTrailColorReset">
            恢复默认颜色
          </ElButton>
        </div>
      </template>
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
    <section>
      <div class="section-head">
        <h2>文本对比模式</h2>
        <label class="row">
          <ElSwitch :model-value="comparePref" @change="onComparePrefChange" />
        </label>
      </div>
      <p>
        开启后可用快捷键进入对比模式：框选并解密两段文本，在中央气泡中左右对比并高亮后段差异。再次按下同一快捷键退出。
      </p>
      <div class="row">
        <span>快捷键</span>
        <div ref="compareButtonRef">
          <ElButton
            :type="compareRecording ? 'primary' : 'default'"
            :disabled="!comparePref"
            @click="startCompareRecording"
            @keydown="onCompareRecordKey"
          >
            {{ compareRecording ? comparePreviewDisplay : compareDisplay }}
          </ElButton>
        </div>
      </div>
      <p v-if="compareErrorMessage" class="error">{{ compareErrorMessage }}</p>
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
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.trail-color-row {
  margin-top: 8px;
}

.trail-shortcut-hint {
  margin-top: 8px;
  font-size: 0.92rem;
}

.trail-key {
  margin-left: 8px;
  color: var(--text-muted);
  font-size: 0.82rem;
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
}

p {
  margin: 8px 0 0;
  color: var(--text-muted);
}

.error {
  color: var(--danger, #c0392b);
}
</style>

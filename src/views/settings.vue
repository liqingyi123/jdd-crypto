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
import {
  DEFAULT_SCREENSAVER_PREF,
  normalizeScreensaverBackground,
  normalizeScreensaverClock,
  normalizeScreensaverPref,
  type ScreensaverBackground,
  type ScreensaverClock,
  type ScreensaverPref,
} from "@/effects/screensaver/types";

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
  defaultShortcut: "Ctrl+Alt+D",
});

const {
  recording: hostsQuickRecording,
  errorMessage: hostsQuickErrorMessage,
  display: hostsQuickDisplay,
  previewDisplay: hostsQuickPreviewDisplay,
  buttonRef: hostsQuickButtonRef,
  startRecording: startHostsQuickRecording,
  cancelRecording: cancelHostsQuickRecording,
  onRecordKey: onHostsQuickRecordKey,
  loadShortcut: loadHostsQuickShortcut,
} = useShortcutRecorder({
  getCommand: "get_hosts_quick_shortcut",
  setCommand: "set_hosts_quick_shortcut",
  defaultShortcut: "Ctrl+Alt+S",
});

const {
  recording: brightnessRecording,
  errorMessage: brightnessErrorMessage,
  display: brightnessDisplay,
  previewDisplay: brightnessPreviewDisplay,
  buttonRef: brightnessButtonRef,
  startRecording: startBrightnessRecording,
  cancelRecording: cancelBrightnessRecording,
  onRecordKey: onBrightnessRecordKey,
  loadShortcut: loadBrightnessShortcut,
} = useShortcutRecorder({
  getCommand: "get_brightness_quick_shortcut",
  setCommand: "set_brightness_quick_shortcut",
  defaultShortcut: "Ctrl+Alt+L",
});

const {
  recording: screensaverRecording,
  errorMessage: screensaverErrorMessage,
  display: screensaverDisplay,
  previewDisplay: screensaverPreviewDisplay,
  buttonRef: screensaverButtonRef,
  startRecording: startScreensaverRecording,
  cancelRecording: cancelScreensaverRecording,
  onRecordKey: onScreensaverRecordKey,
  loadShortcut: loadScreensaverShortcut,
} = useShortcutRecorder({
  getCommand: "get_screensaver_shortcut",
  setCommand: "set_screensaver_shortcut",
  defaultShortcut: "Ctrl+Alt+P",
});

const themeOptions: Array<{ value: ThemePreference; label: string }> = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
];

const badgeSize = shallowRef(DEFAULT_BADGE_SIZE);
const followPref = shallowRef(true);
const comparePref = shallowRef(true);
const hostsQuickPref = shallowRef(true);
const brightnessQuickPref = shallowRef(true);
const screensaverPref = shallowRef(true);
const screensaverBackground = shallowRef<ScreensaverBackground>(
  DEFAULT_SCREENSAVER_PREF.background,
);
const screensaverClock = shallowRef<ScreensaverClock>(DEFAULT_SCREENSAVER_PREF.clock);
const autostartEnabled = shallowRef(false);
const intranetServerBase = shallowRef("http://172.20.2.169:7101/");
const intranetServerSaving = shallowRef(false);
const DEFAULT_INTRANET_SERVER = "http://172.20.2.169:7101/";
const trailEnabled = shallowRef(DEFAULT_MOUSE_TRAIL_PREF.enabled);
const trailEffect = shallowRef<MouseTrailEffect>(DEFAULT_MOUSE_TRAIL_PREF.effect);
const trailColors = shallowRef<MouseTrailColors>({ ...DEFAULT_MOUSE_TRAIL_COLORS });
const committedTrailColors = shallowRef<MouseTrailColors>({ ...DEFAULT_MOUSE_TRAIL_COLORS });
const trailColorsResetting = shallowRef(false);
const activeTab = shallowRef("general");

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
let trailColorPersistTimer: ReturnType<typeof setTimeout> | null = null;
let trailColorPersistToken = 0;

function applyTrailPref(pref: MouseTrailPref) {
  trailEnabled.value = pref.enabled;
  trailEffect.value = normalizeMouseTrailEffect(pref.effect);
  const colors = normalizeMouseTrailColors(pref.colors);
  trailColors.value = colors;
  committedTrailColors.value = { ...colors };
}

function clearTrailColorPersistTimer() {
  if (trailColorPersistTimer !== null) {
    clearTimeout(trailColorPersistTimer);
    trailColorPersistTimer = null;
  }
}

function optimisticTrailColor(effect: ColorableTrailEffect, color: string) {
  trailColors.value = {
    ...trailColors.value,
    [effect]: color,
  };
}

async function persistTrailColor(color: string) {
  const effect = activeColorEffect.value;
  if (!effect) {
    return;
  }
  const rollback = committedTrailColors.value[effect];
  optimisticTrailColor(effect, color);
  const token = ++trailColorPersistToken;
  try {
    const pref = await invoke<MouseTrailPref>("set_mouse_trail_color", { effect, color });
    if (token !== trailColorPersistToken) {
      return;
    }
    applyTrailPref(pref);
  } catch (error) {
    if (token !== trailColorPersistToken) {
      return;
    }
    optimisticTrailColor(effect, rollback);
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

function onTrailColorActiveChange(color: string | null) {
  const effect = activeColorEffect.value;
  if (!color || !effect) {
    return;
  }
  optimisticTrailColor(effect, color);
  clearTrailColorPersistTimer();
  trailColorPersistTimer = setTimeout(() => {
    trailColorPersistTimer = null;
    void persistTrailColor(color);
  }, 100);
}

function onTrailColorChange(color: string | null) {
  if (!color) {
    return;
  }
  clearTrailColorPersistTimer();
  void persistTrailColor(color);
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
    hostsQuickPref.value = await invoke<boolean>("get_hosts_quick_pref");
  } catch {
    // browser preview
  }
  try {
    brightnessQuickPref.value = await invoke<boolean>("get_brightness_quick_pref");
  } catch {
    // browser preview
  }
  try {
    screensaverPref.value = await invoke<boolean>("get_screensaver_pref");
  } catch {
    // browser preview
  }
  try {
    const raw = await invoke<ScreensaverPref>("get_screensaver_effect_pref");
    const normalized = normalizeScreensaverPref(raw);
    screensaverBackground.value = normalized.background;
    screensaverClock.value = normalized.clock;
  } catch {
    screensaverBackground.value = DEFAULT_SCREENSAVER_PREF.background;
    screensaverClock.value = DEFAULT_SCREENSAVER_PREF.clock;
  }
  try {
    autostartEnabled.value = await invoke<boolean>("get_autostart_pref");
  } catch {
    autostartEnabled.value = false;
  }
  try {
    intranetServerBase.value = await invoke<string>("get_intranet_server_base");
  } catch {
    intranetServerBase.value = DEFAULT_INTRANET_SERVER;
  }
  try {
    const pref = await invoke<MouseTrailPref>("get_mouse_trail_pref");
    applyTrailPref(pref);
  } catch {
    applyTrailPref(DEFAULT_MOUSE_TRAIL_PREF);
  }
  await loadShortcut();
  await loadCompareShortcut();
  await loadHostsQuickShortcut();
  await loadBrightnessShortcut();
  await loadScreensaverShortcut();
  try {
    unlistenTrailPref = await listen<MouseTrailPref>("app://mouse-trail-pref", (event) => {
      applyTrailPref(event.payload);
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  clearTrailColorPersistTimer();
  void unlistenTrailPref?.();
});

async function onWatchChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  const previous = clipboardStore.watchEnabled;
  clipboardStore.setWatchEnabled(enabled);
  try {
    await invoke("set_clipboard_watch", { enabled });
  } catch (error) {
    clipboardStore.setWatchEnabled(previous);
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onBadgeSizeChange(value: string | number | boolean | undefined) {
  const size = Number(value);
  if (!Number.isFinite(size)) {
    return;
  }
  const previous = badgeSize.value;
  badgeSize.value = size;
  try {
    await invoke("set_badge_size_pref", { size });
  } catch (error) {
    badgeSize.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onFollowPrefChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  const previous = followPref.value;
  followPref.value = enabled;
  if (!enabled && recording.value) {
    await cancelRecording();
  }
  try {
    await invoke("set_mouse_follow_pref", { enabled });
  } catch (error) {
    followPref.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onComparePrefChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  const previous = comparePref.value;
  comparePref.value = enabled;
  if (!enabled && compareRecording.value) {
    await cancelCompareRecording();
  }
  try {
    await invoke("set_compare_mode_pref", { enabled });
  } catch (error) {
    comparePref.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onHostsQuickPrefChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  const previous = hostsQuickPref.value;
  hostsQuickPref.value = enabled;
  if (!enabled && hostsQuickRecording.value) {
    await cancelHostsQuickRecording();
  }
  try {
    await invoke("set_hosts_quick_pref", { enabled });
  } catch (error) {
    hostsQuickPref.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onBrightnessQuickPrefChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  const previous = brightnessQuickPref.value;
  brightnessQuickPref.value = enabled;
  if (!enabled && brightnessRecording.value) {
    await cancelBrightnessRecording();
  }
  try {
    await invoke("set_brightness_quick_pref", { enabled });
  } catch (error) {
    brightnessQuickPref.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onScreensaverPrefChange(value: string | number | boolean) {
  const enabled = Boolean(value);
  const previous = screensaverPref.value;
  screensaverPref.value = enabled;
  if (!enabled && screensaverRecording.value) {
    await cancelScreensaverRecording();
  }
  try {
    await invoke("set_screensaver_pref", { enabled });
  } catch (error) {
    screensaverPref.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onScreensaverBackgroundChange(
  value: string | number | boolean | undefined,
) {
  const background = normalizeScreensaverBackground(value);
  const previous = screensaverBackground.value;
  screensaverBackground.value = background;
  try {
    const pref = await invoke<ScreensaverPref>("set_screensaver_background", {
      background,
    });
    const normalized = normalizeScreensaverPref(pref);
    screensaverBackground.value = normalized.background;
    screensaverClock.value = normalized.clock;
  } catch (error) {
    screensaverBackground.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function onScreensaverClockChange(
  value: string | number | boolean | undefined,
) {
  const clock = normalizeScreensaverClock(value);
  const previous = screensaverClock.value;
  screensaverClock.value = clock;
  try {
    const pref = await invoke<ScreensaverPref>("set_screensaver_clock", {
      clock,
    });
    const normalized = normalizeScreensaverPref(pref);
    screensaverBackground.value = normalized.background;
    screensaverClock.value = normalized.clock;
  } catch (error) {
    screensaverClock.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
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

async function persistIntranetServerBase(raw: string) {
  const previous = intranetServerBase.value;
  intranetServerSaving.value = true;
  try {
    const saved = await invoke<string>("set_intranet_server_base", { base: raw });
    intranetServerBase.value = saved;
    ElMessage.success("服务器地址已保存");
  } catch (error) {
    intranetServerBase.value = previous;
    ElMessage.error(error instanceof Error ? error.message : String(error));
  } finally {
    intranetServerSaving.value = false;
  }
}

async function onIntranetServerSave() {
  await persistIntranetServerBase(intranetServerBase.value);
}

async function onIntranetServerReset() {
  await persistIntranetServerBase(DEFAULT_INTRANET_SERVER);
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

async function onTrailColorReset() {
  const effect = activeColorEffect.value;
  if (!effect) {
    return;
  }
  clearTrailColorPersistTimer();
  trailColorPersistToken += 1;
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

async function onThemeChange(value: string | number | boolean | undefined) {
  if (value !== "system" && value !== "light" && value !== "dark") {
    return;
  }
  const previous = themeStore.preference;
  themeStore.setPreference(value);
  try {
    await invoke("set_theme_pref", { preference: value });
  } catch (error) {
    themeStore.setPreference(previous);
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}
</script>

<template>
  <div class="page">
    <ElTabs v-model="activeTab" class="settings-tabs">
      <ElTabPane label="常规" name="general">
        <div class="tab-panels">
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
            <p>开启后登录系统时自动启动多多工具箱。</p>
          </section>
          <section>
            <h2>内网服务器</h2>
            <p>
              用于检查更新与拉取 Hosts 预置配置。默认
              <code>{{ DEFAULT_INTRANET_SERVER }}</code>，保存后重启仍生效。
            </p>
            <div class="row intranet-row">
              <ElInput
                v-model="intranetServerBase"
                clearable
                placeholder="http://172.20.2.169:7101/"
                @keyup.enter="onIntranetServerSave"
              />
              <ElButton
                type="primary"
                :loading="intranetServerSaving"
                @click="onIntranetServerSave"
              >
                保存
              </ElButton>
              <ElButton :disabled="intranetServerSaving" @click="onIntranetServerReset">
                恢复默认
              </ElButton>
            </div>
          </section>
        </div>
      </ElTabPane>

      <ElTabPane label="加解密" name="crypto">
        <div class="tab-panels">
          <section>
            <div class="section-head">
              <h2>剪贴板</h2>
              <label class="row">
                <ElSwitch :model-value="watchEnabled" @change="onWatchChange" />
              </label>
            </div>
            <p>
              自动识别剪贴板并静默尝试解密（仅 JSON 结果弹气泡），关闭后不再轮询剪贴板，避免打扰与隐私风险。
            </p>
          </section>
          <section>
            <div class="section-head">
              <h2>鼠标跟随</h2>
              <label class="row">
                <ElSwitch :model-value="followPref" @change="onFollowPrefChange" />
              </label>
            </div>
            <p>
              按下快捷键开启后会跟随鼠标移动并自动等待鼠标下次的文本选中，按下鼠标选中文本后松开会自动打开加解密页面并代入选中的文本。
            </p>
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
      </ElTabPane>

      <ElTabPane label="系统易用性" name="usability">
        <div class="tab-panels">
          <section>
            <div class="section-head">
              <h2>鼠标轨迹特效</h2>
              <label class="row">
                <ElSwitch :model-value="trailEnabled" @change="onTrailEnabledChange" />
              </label>
            </div>
            <p>
              我的鼠标指针哪去啦？？！！<br />在所有显示器工作区跟随鼠标绘制炫酷好玩的拖尾特效以帮助更好的寻找鼠标位置。
            </p>
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
                  :clearable="false"
                  popper-class="trail-color-picker"
                  @active-change="onTrailColorActiveChange"
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
              <h2>屏幕亮度调节</h2>
              <label class="row">
                <ElSwitch
                  :model-value="brightnessQuickPref"
                  @change="onBrightnessQuickPrefChange"
                />
              </label>
            </div>
            <p>
              按下快捷键后在鼠标旁打开亮度气泡，为每个显示器显示进度条，拖动即可调节该屏物理亮度。
            </p>
            <div class="row">
              <span>快捷键</span>
              <div ref="brightnessButtonRef">
                <ElButton
                  :type="brightnessRecording ? 'primary' : 'default'"
                  :disabled="!brightnessQuickPref"
                  @click="startBrightnessRecording"
                  @keydown="onBrightnessRecordKey"
                >
                  {{
                    brightnessRecording
                      ? brightnessPreviewDisplay
                      : brightnessDisplay
                  }}
                </ElButton>
              </div>
            </div>
            <p v-if="brightnessErrorMessage" class="error">
              {{ brightnessErrorMessage }}
            </p>
            <p>最多 4 个键，需包含修饰键。点击按钮后按下新组合，Esc 或点击其他区域取消。</p>
          </section>
          <section>
            <div class="section-head">
              <h2>屏幕保护</h2>
              <label class="row">
                <ElSwitch
                  :model-value="screensaverPref"
                  @change="onScreensaverPrefChange"
                />
              </label>
            </div>
            <p>
              按下快捷键后在所有显示器全屏显示屏幕保护特效；按任意键、点击鼠标或再次快捷键退出。开启期间会暂时关闭鼠标轨迹。
            </p>
            <div class="row">
              <span>快捷键</span>
              <div ref="screensaverButtonRef">
                <ElButton
                  :type="screensaverRecording ? 'primary' : 'default'"
                  :disabled="!screensaverPref"
                  @click="startScreensaverRecording"
                  @keydown="onScreensaverRecordKey"
                >
                  {{
                    screensaverRecording
                      ? screensaverPreviewDisplay
                      : screensaverDisplay
                  }}
                </ElButton>
              </div>
            </div>
            <p v-if="screensaverErrorMessage" class="error">
              {{ screensaverErrorMessage }}
            </p>
            <p>最多 4 个键，需包含修饰键。点击按钮后按下新组合，Esc 或点击其他区域取消。</p>
            <h3>背景特效</h3>
            <ElRadioGroup
              :model-value="screensaverBackground"
              :disabled="!screensaverPref"
              @change="onScreensaverBackgroundChange"
            >
              <ElRadio value="parallax">海景视差</ElRadio>
              <ElRadio value="corona">日冕生辉</ElRadio>
              <ElRadio value="snow">雪国遗踪</ElRadio>
            </ElRadioGroup>
            <h3>时钟效果</h3>
            <ElRadioGroup
              :model-value="screensaverClock"
              :disabled="!screensaverPref"
              @change="onScreensaverClockChange"
            >
              <ElRadio value="lcd3d">3D液晶时钟</ElRadio>
              <ElRadio value="analog">经典指针</ElRadio>
              <ElRadio value="compass">文字罗盘</ElRadio>
              <ElRadio value="off">关闭</ElRadio>
            </ElRadioGroup>
          </section>
        </div>
      </ElTabPane>

      <ElTabPane label="Host管理" name="hosts">
        <div class="tab-panels">
          <section>
            <div class="section-head">
              <h2>Host 快速切换</h2>
              <label class="row">
                <ElSwitch :model-value="hostsQuickPref" @change="onHostsQuickPrefChange" />
              </label>
            </div>
            <p>
              按下快捷键后在鼠标旁打开 Host 方案列表，切换开关后窗口自动关闭。
            </p>
            <div class="row">
              <span>快捷键</span>
              <div ref="hostsQuickButtonRef">
                <ElButton
                  :type="hostsQuickRecording ? 'primary' : 'default'"
                  :disabled="!hostsQuickPref"
                  @click="startHostsQuickRecording"
                  @keydown="onHostsQuickRecordKey"
                >
                  {{
                    hostsQuickRecording
                      ? hostsQuickPreviewDisplay
                      : hostsQuickDisplay
                  }}
                </ElButton>
              </div>
            </div>
            <p v-if="hostsQuickErrorMessage" class="error">
              {{ hostsQuickErrorMessage }}
            </p>
            <p>最多 4 个键，需包含修饰键。点击按钮后按下新组合，Esc 或点击其他区域取消。</p>
          </section>
        </div>
      </ElTabPane>
    </ElTabs>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  height: 100%;
}

.settings-tabs {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.settings-tabs :deep(.el-tabs__header) {
  flex-shrink: 0;
  margin: 0 0 12px;
}

.settings-tabs :deep(.el-tabs__nav-wrap) {
  margin-bottom: 0;
}

.settings-tabs :deep(.el-tabs__item) {
  height: 36px;
  line-height: 36px;
  padding: 0 16px;
}

.settings-tabs :deep(.el-tabs__content) {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding-bottom: 8px;
}

.settings-tabs :deep(.el-tab-pane) {
  height: 100%;
}

.tab-panels {
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

.intranet-row {
  margin-top: 12px;
  align-items: stretch;
}

.intranet-row :deep(.el-input) {
  flex: 1;
  min-width: 0;
}

code {
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
  font-size: 0.9em;
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

<!-- ColorPicker 面板 teleport 到 body；EP 2.x 确认按钮类名为 el-color-footer__btn -->
<style>
.trail-color-picker .el-color-footer__btn,
.trail-color-picker .el-color-footer__link-btn {
  display: none !important;
}
</style>

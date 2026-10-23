<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSystemTheme } from "@/composables/use-system-theme";
import fanqieIcon from "@/assets/image/fanqie.png";
import qqreadIcon from "@/assets/image/qqread.png";
import wxreadIcon from "@/assets/image/wxread.png";

useSystemTheme();

const HOME_URL_KEY = "miniWindowHomeUrl";

interface MiniWindowPrefs {
  autoHide: boolean;
  alwaysOnTop: boolean;
  contentOpacity: number;
  backgroundOpacity: number;
  browsing: boolean;
}

const PRESETS = [
  {
    id: "fanqie",
    label: "番茄小说",
    url: "https://fanqienovel.com/",
    icon: fanqieIcon,
  },
  {
    id: "qqread",
    label: "QQ阅读",
    url: "https://book.qq.com/",
    icon: qqreadIcon,
  },
  {
    id: "wxread",
    label: "微信读书",
    url: "https://weread.qq.com/",
    icon: wxreadIcon,
  },
] as const;

const browsing = shallowRef(false);
const autoHide = shallowRef(false);
const alwaysOnTop = shallowRef(true);
const contentOpacity = shallowRef(1);
const backgroundOpacity = shallowRef(1);
const opacityOpen = shallowRef(false);
const urlInput = shallowRef("");
const busy = shallowRef(false);
const urlError = shallowRef("");

let unlistenPrefs: UnlistenFn | undefined;
let unlistenBrowsing: UnlistenFn | undefined;
let unlistenContentOpacity: UnlistenFn | undefined;
let unlistenBackgroundOpacity: UnlistenFn | undefined;
let unlistenOpacityClosed: UnlistenFn | undefined;
let unlistenOpacityOpened: UnlistenFn | undefined;
let unlistenOpacityFailed: UnlistenFn | undefined;

/** Background opacity is parent LWA — do not also fade the shell via CSS. */
const contentStyle = computed(() => ({
  opacity: contentOpacity.value,
}));

function applyPrefs(prefs: MiniWindowPrefs) {
  autoHide.value = Boolean(prefs.autoHide);
  alwaysOnTop.value = Boolean(prefs.alwaysOnTop);
  contentOpacity.value = Number(prefs.contentOpacity) || 1;
  backgroundOpacity.value = Number(prefs.backgroundOpacity) || 1;
  browsing.value = Boolean(prefs.browsing);
}

async function setDragging(dragging: boolean) {
  await invoke("mini_window_set_dragging", { dragging }).catch(() => undefined);
}

async function hideOpacityBubble() {
  opacityOpen.value = false;
  await invoke("hide_mini_opacity_bubble").catch(() => undefined);
}

async function toggleOpacityBubble(event: MouseEvent) {
  if (opacityOpen.value) {
    await hideOpacityBubble();
    return;
  }
  const btn = event.currentTarget as HTMLElement | null;
  if (!btn) {
    return;
  }
  try {
    const win = getCurrentWindow();
    const [outer, scale] = await Promise.all([win.outerPosition(), win.scaleFactor()]);
    const rect = btn.getBoundingClientRect();
    const x = Math.round(outer.x + rect.left * scale);
    const y = Math.round(outer.y + (rect.bottom + 6) * scale);
    // Rust creates the bubble async; wait for opacity-opened / opacity-failed.
    await invoke("show_mini_opacity_bubble", { x, y });
  } catch (error) {
    console.error(error);
    opacityOpen.value = false;
  }
}

function endDrag() {
  void setDragging(false);
}

async function startDrag(event: MouseEvent) {
  if (event.button !== 0) {
    return;
  }
  const target = event.target as HTMLElement | null;
  if (target?.closest("button, input, a, .resize-handle")) {
    return;
  }
  // On Windows startDragging often resolves immediately — do not clear
  // dragging here; mouseup + Rust Moved idle-clear end the session.
  await setDragging(true);
  await getCurrentWindow().startDragging().catch(() => undefined);
}

type ResizeDir =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

async function startResize(event: MouseEvent, direction: ResizeDir) {
  if (event.button !== 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  await setDragging(true);
  await getCurrentWindow().startResizeDragging(direction).catch(() => undefined);
}

async function goBack() {
  await invoke("mini_window_go_back").catch(() => undefined);
}

async function goForward() {
  await invoke("mini_window_go_forward").catch(() => undefined);
}

async function toggleAutoHide() {
  const next = !autoHide.value;
  autoHide.value = next;
  await invoke("set_mini_window_auto_hide", { enabled: next }).catch(() => {
    autoHide.value = !next;
  });
}

async function toggleAlwaysOnTop() {
  const next = !alwaysOnTop.value;
  alwaysOnTop.value = next;
  await invoke("set_mini_window_always_on_top", { enabled: next }).catch(() => {
    alwaysOnTop.value = !next;
  });
}

async function minimize() {
  await hideOpacityBubble();
  await invoke("mini_window_minimize").catch(() => undefined);
}

async function closeWindow() {
  await hideOpacityBubble();
  await invoke("mini_window_close").catch(() => undefined);
}

/** Reject bare numbers / hostnames without a dot (e.g. `4532543`). */
function isLikelyWebUrl(raw: string): boolean {
  const trimmed = raw.trim();
  if (!trimmed) {
    return false;
  }
  // Chromium/url treat a bare decimal host as IPv4 (15646 → 0.0.61.14). Reject that.
  const withoutScheme = trimmed.replace(/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//, "");
  const hostToken = withoutScheme.split(/[/:?#]/)[0] ?? "";
  if (/^\d+$/.test(hostToken)) {
    return false;
  }
  try {
    const withScheme = /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(trimmed)
      ? trimmed
      : `https://${trimmed}`;
    const parsed = new URL(withScheme);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      return false;
    }
    const host = parsed.hostname;
    if (!host) {
      return false;
    }
    if (host === "localhost" || host.endsWith(".localhost")) {
      return true;
    }
    // Only accept dotted IPv4 that the user actually typed (not integer form).
    if (/^\d{1,3}(\.\d{1,3}){3}$/.test(host)) {
      return /^\d{1,3}(\.\d{1,3}){3}$/.test(hostToken);
    }
    if (host.startsWith("[") && host.endsWith("]")) {
      return true;
    }
    return host.includes(".");
  } catch {
    return false;
  }
}

function rememberHomeUrl(url: string) {
  try {
    localStorage.setItem(HOME_URL_KEY, url);
  } catch {
    // private mode / storage full
  }
}

async function openUrl(raw: string, options?: { remember?: boolean }) {
  const url = raw.trim();
  if (!url || busy.value) {
    return;
  }
  if (!isLikelyWebUrl(url)) {
    urlError.value = "请输入有效网址";
    // Invalid input must not overwrite the remembered URL.
    return;
  }
  urlError.value = "";
  if (options?.remember) {
    rememberHomeUrl(url);
  }
  busy.value = true;
  try {
    await hideOpacityBubble();
    await invoke("mini_window_navigate", { url });
    browsing.value = true;
  } catch (error) {
    console.error(error);
    urlError.value = typeof error === "string" ? error : "打开失败";
  } finally {
    busy.value = false;
  }
}

async function openPreset(url: string) {
  urlError.value = "";
  // Presets open directly and must not overwrite the remembered input URL.
  await openUrl(url);
}

async function submitUrl() {
  await openUrl(urlInput.value, { remember: true });
}

function clearUrlInput() {
  urlInput.value = "";
  urlError.value = "";
}

async function goHome() {
  await invoke("mini_window_show_home").catch(() => undefined);
  browsing.value = false;
}

watch(urlInput, () => {
  if (urlError.value) {
    urlError.value = "";
  }
});

onMounted(async () => {
  window.addEventListener("mouseup", endDrag);
  try {
    urlInput.value = localStorage.getItem(HOME_URL_KEY) ?? "";
  } catch {
    // private mode
  }
  try {
    const prefs = await invoke<MiniWindowPrefs>("get_mini_window_prefs");
    applyPrefs(prefs);
  } catch {
    // browser preview
  }
  try {
    unlistenPrefs = await listen<MiniWindowPrefs>("mini-window://prefs", (event) => {
      applyPrefs(event.payload);
    });
    unlistenBrowsing = await listen<boolean>("mini-window://browsing", (event) => {
      browsing.value = Boolean(event.payload);
    });
    unlistenContentOpacity = await listen<number>(
      "mini-window://content-opacity",
      (event) => {
        contentOpacity.value = Number(event.payload) || 1;
      },
    );
    unlistenBackgroundOpacity = await listen<number>(
      "mini-window://background-opacity",
      (event) => {
        backgroundOpacity.value = Number(event.payload) || 1;
      },
    );
    unlistenOpacityClosed = await listen("mini-window://opacity-closed", () => {
      opacityOpen.value = false;
    });
    unlistenOpacityOpened = await listen("mini-window://opacity-opened", () => {
      opacityOpen.value = true;
    });
    unlistenOpacityFailed = await listen("mini-window://opacity-failed", () => {
      opacityOpen.value = false;
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  window.removeEventListener("mouseup", endDrag);
  endDrag();
  void hideOpacityBubble();
  void unlistenPrefs?.();
  void unlistenBrowsing?.();
  void unlistenContentOpacity?.();
  void unlistenBackgroundOpacity?.();
  void unlistenOpacityClosed?.();
  void unlistenOpacityOpened?.();
  void unlistenOpacityFailed?.();
});
</script>

<template>
  <div class="mini-shell" :class="{ browsing }">
    <div
      class="resize-handle n"
      @mousedown="startResize($event, 'North')"
    />
    <div
      class="resize-handle s"
      @mousedown="startResize($event, 'South')"
    />
    <div
      class="resize-handle e"
      @mousedown="startResize($event, 'East')"
    />
    <div
      class="resize-handle w"
      @mousedown="startResize($event, 'West')"
    />
    <div
      class="resize-handle ne"
      @mousedown="startResize($event, 'NorthEast')"
    />
    <div
      class="resize-handle nw"
      @mousedown="startResize($event, 'NorthWest')"
    />
    <div
      class="resize-handle se"
      @mousedown="startResize($event, 'SouthEast')"
    />
    <div
      class="resize-handle sw"
      @mousedown="startResize($event, 'SouthWest')"
    />
    <header class="titlebar" @mousedown="startDrag">
      <div class="titlebar-left">
        <button
          type="button"
          class="icon-btn"
          title="后退"
          @mousedown.stop
          @click="goBack"
        >
          <span class="chevron left" />
        </button>
        <button
          type="button"
          class="icon-btn"
          title="前进"
          @mousedown.stop
          @click="goForward"
        >
          <span class="chevron right" />
        </button>
        <button
          v-if="browsing"
          type="button"
          class="text-btn"
          title="返回首页"
          @mousedown.stop
          @click="goHome"
        >
          首页
        </button>
      </div>
      <div class="titlebar-right">
        <button
          type="button"
          class="icon-btn"
          :class="{ active: autoHide }"
          :title="autoHide ? '移出隐藏：开' : '移出隐藏：关'"
          @mousedown.stop
          @click="toggleAutoHide"
        >
          <svg
            v-if="autoHide"
            class="tb-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              d="M3 3l18 18M10.6 10.6a3 3 0 0 0 4.2 4.2M9.9 5.1A10.5 10.5 0 0 1 12 5c7 0 10 7 10 7a17 17 0 0 1-3.2 4.1M6.1 6.1A17 17 0 0 0 2 12s3 7 10 7a10.3 10.3 0 0 0 4.4-1"
              fill="none"
              stroke="currentColor"
              stroke-width="2.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <svg
            v-else
            class="tb-icon tb-icon-off"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z"
              fill="none"
              stroke="currentColor"
              stroke-width="2.4"
              stroke-linejoin="round"
            />
            <circle
              cx="12"
              cy="12"
              r="3"
              fill="none"
              stroke="currentColor"
              stroke-width="2.4"
            />
          </svg>
        </button>
        <button
          type="button"
          class="icon-btn"
          :class="{ active: opacityOpen }"
          title="透明度"
          @mousedown.stop
          @click.stop="toggleOpacityBubble"
        >
          <span class="sliders" />
        </button>
        <button
          type="button"
          class="icon-btn"
          :class="{ active: alwaysOnTop }"
          :title="alwaysOnTop ? '取消置顶' : '窗口置顶'"
          @mousedown.stop
          @click="toggleAlwaysOnTop"
        >
          <svg
            class="tb-icon"
            :class="{ 'tb-icon-off': !alwaysOnTop }"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              d="M12 17v5M9 2h6M15 2v1a3 3 0 0 1 0 6v1l3 3H6l3-3V9a3 3 0 0 1 0-6V2"
              fill="none"
              stroke="currentColor"
              stroke-width="2.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>
        <button
          type="button"
          class="icon-btn"
          title="最小化"
          @mousedown.stop
          @click.stop="minimize"
        >
          <span class="minimize" />
        </button>
        <button
          type="button"
          class="icon-btn close"
          title="关闭"
          @mousedown.stop
          @click.stop="closeWindow"
        >
          <span class="close-x" />
        </button>
      </div>
    </header>

    <main v-if="!browsing" class="home" :style="contentStyle">
      <div class="home-hero">
        <div class="home-icon" aria-hidden="true">
          <span class="plus" />
        </div>
        <p class="home-title">在迷你窗中打开网页</p>
        <p class="home-sub">选择下方内容，或输入任意网址</p>
      </div>

      <div class="divider"><span>或添加以下内容</span></div>

      <div class="preset-row">
        <button
          v-for="item in PRESETS"
          :key="item.id"
          type="button"
          class="preset-card"
          :disabled="busy"
          @click="openPreset(item.url)"
        >
          <img class="preset-glyph" :src="item.icon" :alt="item.label" />
          <span>{{ item.label }}</span>
        </button>
      </div>

      <form class="url-form" @submit.prevent="submitUrl">
        <div class="url-field">
          <input
            v-model="urlInput"
            type="text"
            class="url-input"
            :class="{
              'has-clear': Boolean(urlInput.trim()),
              invalid: Boolean(urlError),
            }"
            placeholder="输入网址，回车打开"
            spellcheck="false"
          />
          <button
            v-if="urlInput.trim()"
            type="button"
            class="url-clear"
            title="清空"
            aria-label="清空"
            @click="clearUrlInput"
          >
            ×
          </button>
        </div>
        <button type="submit" class="url-go" :disabled="busy || !urlInput.trim()">
          打开
        </button>
      </form>
      <p v-if="urlError" class="url-error">{{ urlError }}</p>
    </main>

    <div v-else class="browse-pad" aria-hidden="true" />
  </div>
</template>

<style scoped>
.mini-shell {
  position: relative;
  box-sizing: border-box;
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  border-radius: 14px;
  overflow: hidden;
  background: linear-gradient(180deg, #3a322c 0%, #2a241f 100%);
  border: 1px solid rgba(255, 214, 186, 0.18);
  color: #f3d9c4;
  user-select: none;
}

.mini-shell.browsing {
  background: rgba(42, 36, 31, 0.92);
}

.resize-handle {
  position: absolute;
  z-index: 50;
  background: transparent;
}

.resize-handle.n {
  top: 0;
  left: 6px;
  right: 6px;
  height: 6px;
  cursor: n-resize;
}

.resize-handle.s {
  bottom: 0;
  left: 6px;
  right: 6px;
  height: 6px;
  cursor: s-resize;
}

.resize-handle.e {
  top: 6px;
  right: 0;
  bottom: 6px;
  width: 6px;
  cursor: e-resize;
}

.resize-handle.w {
  top: 6px;
  left: 0;
  bottom: 6px;
  width: 6px;
  cursor: w-resize;
}

.resize-handle.ne {
  top: 0;
  right: 0;
  width: 10px;
  height: 10px;
  cursor: ne-resize;
}

.resize-handle.nw {
  top: 0;
  left: 0;
  width: 10px;
  height: 10px;
  cursor: nw-resize;
}

.resize-handle.se {
  bottom: 0;
  right: 0;
  width: 10px;
  height: 10px;
  cursor: se-resize;
}

.resize-handle.sw {
  bottom: 0;
  left: 0;
  width: 10px;
  height: 10px;
  cursor: sw-resize;
}

.titlebar {
  height: 44px;
  flex: 0 0 44px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 0 6px;
  gap: 4px;
}

.titlebar-left,
.titlebar-right {
  display: flex;
  align-items: center;
  gap: 2px;
}

.icon-btn {
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: #f0c9ad;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  position: relative;
}

.icon-btn:hover,
.icon-btn.active {
  background: rgba(255, 214, 186, 0.12);
}

.icon-btn.close:hover {
  background: rgba(220, 80, 80, 0.35);
  color: #fff;
}

.text-btn {
  height: 28px;
  padding: 0 8px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: #f0c9ad;
  font-size: 12px;
  cursor: pointer;
}

.text-btn:hover {
  background: rgba(255, 214, 186, 0.12);
}

.chevron {
  width: 8px;
  height: 8px;
  border-top: 2px solid currentColor;
  border-right: 2px solid currentColor;
  display: block;
}

.chevron.left {
  transform: rotate(-135deg);
  margin-left: 2px;
}

.chevron.right {
  transform: rotate(45deg);
  margin-right: 2px;
}

.tb-icon {
  display: block;
  width: 14px;
  height: 14px;
}

.tb-icon-off {
  opacity: 0.45;
}

.sliders {
  width: 12px;
  height: 12px;
  position: relative;
  display: block;
}

.sliders::before,
.sliders::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 2px;
  background: currentColor;
  border-radius: 1px;
}

.sliders::before {
  top: 3px;
}

.sliders::after {
  bottom: 3px;
}

.minimize {
  width: 11px;
  height: 2px;
  background: currentColor;
  display: block;
  border-radius: 1px;
}

.close-x {
  width: 12px;
  height: 12px;
  position: relative;
  display: block;
}

.close-x::before,
.close-x::after {
  content: "";
  position: absolute;
  left: 5px;
  top: 0;
  width: 2px;
  height: 12px;
  background: currentColor;
  border-radius: 1px;
}

.close-x::before {
  transform: rotate(45deg);
}

.close-x::after {
  transform: rotate(-45deg);
}

.home {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 16px 18px;
  gap: 14px;
}

.home-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  margin-top: 18px;
}

.home-icon {
  width: 72px;
  height: 56px;
  border: 2px solid rgba(240, 201, 173, 0.55);
  border-radius: 10px;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.home-icon::before {
  content: "";
  position: absolute;
  left: 8px;
  right: 8px;
  top: 8px;
  height: 2px;
  background: rgba(240, 201, 173, 0.45);
}

.plus {
  width: 18px;
  height: 18px;
  position: relative;
}

.plus::before,
.plus::after {
  content: "";
  position: absolute;
  background: #f0c9ad;
  border-radius: 1px;
}

.plus::before {
  left: 8px;
  top: 0;
  width: 2px;
  height: 18px;
}

.plus::after {
  left: 0;
  top: 8px;
  width: 18px;
  height: 2px;
}

.home-title {
  margin: 8px 0 0;
  font-size: 14px;
  font-weight: 600;
  text-align: center;
}

.home-sub {
  margin: 0;
  font-size: 12px;
  opacity: 0.7;
  text-align: center;
}

.divider {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  color: rgba(243, 217, 196, 0.55);
  font-size: 12px;
}

.divider::before,
.divider::after {
  content: "";
  flex: 1;
  height: 1px;
  background: rgba(243, 217, 196, 0.22);
}

.preset-row {
  display: flex;
  gap: 12px;
  width: 100%;
  justify-content: center;
}

.preset-card {
  width: 84px;
  height: 84px;
  border-radius: 14px;
  border: 1px solid rgba(255, 214, 186, 0.16);
  background: rgba(255, 214, 186, 0.06);
  color: #f3d9c4;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  cursor: pointer;
  font-size: 12px;
}

.preset-card:hover:not(:disabled) {
  background: rgba(255, 214, 186, 0.12);
}

.preset-card:disabled {
  opacity: 0.5;
  cursor: default;
}

.preset-glyph {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  object-fit: contain;
}

.url-form {
  width: 100%;
  display: flex;
  gap: 8px;
  margin-top: auto;
}

.url-error {
  margin: 6px 0 0;
  font-size: 11px;
  color: #e8a0a0;
  line-height: 1.3;
}

.url-field {
  position: relative;
  flex: 1;
  min-width: 0;
}

.url-input {
  box-sizing: border-box;
  width: 100%;
  height: 34px;
  border-radius: 10px;
  border: 1px solid rgba(255, 214, 186, 0.2);
  background: rgba(0, 0, 0, 0.22);
  color: #f3d9c4;
  padding: 0 10px;
  outline: none;
  font-size: 12px;
}

.url-input.has-clear {
  padding-right: 30px;
}

.url-input.invalid {
  border-color: rgba(220, 100, 100, 0.55);
}

.url-input::placeholder {
  color: rgba(243, 217, 196, 0.45);
}

.url-clear {
  position: absolute;
  top: 50%;
  right: 6px;
  transform: translateY(-50%);
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: rgba(243, 217, 196, 0.55);
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.url-clear:hover {
  color: #f3d9c4;
  background: rgba(255, 214, 186, 0.12);
}

.url-go {
  height: 34px;
  padding: 0 12px;
  border: 0;
  border-radius: 10px;
  background: rgba(240, 201, 173, 0.22);
  color: #f3d9c4;
  cursor: pointer;
  font-size: 12px;
}

.url-go:disabled {
  opacity: 0.45;
  cursor: default;
}

.browse-pad {
  flex: 1;
  min-height: 0;
  background: transparent;
}
</style>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, shallowRef, useTemplateRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { InputInstance } from "element-plus";
import ParallaxSeaside from "@/effects/screensaver/parallax-seaside.vue";
import CoronaSunrise from "@/effects/screensaver/corona-sunrise.vue";
import SnowRelic from "@/effects/screensaver/snow-relic.vue";
import StarCosmos from "@/effects/screensaver/star-cosmos.vue";
import FluorescentClock from "@/effects/screensaver/fluorescent-clock.vue";
import ClassicAnalogClock from "@/effects/screensaver/classic-analog-clock.vue";
import TextCompassClock from "@/effects/screensaver/text-compass-clock.vue";
import {
  DEFAULT_SCREENSAVER_PREF,
  normalizeScreensaverPref,
  type ScreensaverPref,
} from "@/effects/screensaver/types";

const pref = shallowRef<ScreensaverPref>({ ...DEFAULT_SCREENSAVER_PREF });
const passwordRequired = shallowRef(false);
const unlockVisible = shallowRef(false);
const unlockPassword = shallowRef("");
const unlockError = shallowRef("");
const unlocking = shallowRef(false);
const unlockInputRef = useTemplateRef<InputInstance>("unlockInputRef");

let unlistenPref: UnlistenFn | null = null;
let unlistenPassword: UnlistenFn | null = null;
let unlistenUnlock: UnlistenFn | null = null;

const hintText = computed(() =>
  passwordRequired.value
    ? "按任意键或点击鼠标输入密码退出"
    : "按任意键或点击鼠标退出",
);

async function refreshPasswordRequired() {
  try {
    passwordRequired.value = await invoke<boolean>("screensaver_password_required");
  } catch {
    passwordRequired.value = false;
  }
}

async function exitScreensaver() {
  try {
    await invoke("hide_screensaver");
  } catch {
    // browser preview
  }
}

function openUnlock() {
  unlockPassword.value = "";
  unlockError.value = "";
  unlockVisible.value = true;
  void nextTick(() => {
    unlockInputRef.value?.focus?.();
    unlockInputRef.value?.input?.focus?.();
  });
}

function closeUnlock() {
  unlockVisible.value = false;
  unlockPassword.value = "";
  unlockError.value = "";
}

async function submitUnlock() {
  if (unlocking.value) {
    return;
  }
  unlocking.value = true;
  unlockError.value = "";
  try {
    await invoke("unlock_screensaver", { password: unlockPassword.value });
    closeUnlock();
  } catch (error) {
    unlockError.value = error instanceof Error ? error.message : String(error);
    unlockPassword.value = "";
    void nextTick(() => {
      unlockInputRef.value?.focus?.();
      unlockInputRef.value?.input?.focus?.();
    });
  } finally {
    unlocking.value = false;
  }
}

function requestExit() {
  if (unlockVisible.value) {
    return;
  }
  if (passwordRequired.value) {
    openUnlock();
    return;
  }
  void exitScreensaver();
}

function onKeyDown(event: KeyboardEvent) {
  if (unlockVisible.value) {
    return;
  }
  // Avoid reopening unlock when Esc closes the dialog.
  if (passwordRequired.value && event.key === "Escape") {
    return;
  }
  event.preventDefault();
  requestExit();
}

function onPointerDown(event: PointerEvent) {
  const target = event.target;
  if (target instanceof Element && target.closest(".el-overlay, .el-dialog")) {
    return;
  }
  requestExit();
}

onMounted(async () => {
  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("pointerdown", onPointerDown);
  try {
    const raw = await invoke<unknown>("get_screensaver_effect_pref");
    pref.value = normalizeScreensaverPref(raw);
  } catch {
    pref.value = { ...DEFAULT_SCREENSAVER_PREF };
  }
  await refreshPasswordRequired();
  try {
    unlistenPref = await listen<unknown>("app://screensaver-pref", (event) => {
      pref.value = normalizeScreensaverPref(event.payload);
    });
  } catch {
    // browser preview
  }
  try {
    unlistenPassword = await listen<boolean>(
      "app://screensaver-password-required",
      (event) => {
        passwordRequired.value = Boolean(event.payload);
      },
    );
  } catch {
    // browser preview
  }
  try {
    unlistenUnlock = await listen("app://screensaver-unlock-request", () => {
      openUnlock();
    });
  } catch {
    // browser preview
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("pointerdown", onPointerDown);
  void unlistenPref?.();
  void unlistenPassword?.();
  void unlistenUnlock?.();
});
</script>

<template>
  <div class="screensaver-root">
    <ParallaxSeaside v-if="pref.background === 'parallax'" />
    <CoronaSunrise v-else-if="pref.background === 'corona'" />
    <SnowRelic v-else-if="pref.background === 'snow'" />
    <StarCosmos v-else-if="pref.background === 'star'" />
    <FluorescentClock v-if="pref.clock === 'lcd3d'" />
    <ClassicAnalogClock v-else-if="pref.clock === 'analog'" />
    <TextCompassClock v-else-if="pref.clock === 'compass'" />
    <p class="hint">{{ hintText }}</p>

    <ElDialog
      v-model="unlockVisible"
      title="输入屏保密码"
      width="340px"
      align-center
      append-to-body
      :show-close="false"
      :close-on-click-modal="false"
      :close-on-press-escape="true"
      destroy-on-close
      class="screensaver-unlock-dialog"
      modal-class="screensaver-unlock-modal"
      @closed="closeUnlock"
    >
      <div class="unlock-body" @keydown.stop @pointerdown.stop>
        <ElInput
          ref="unlockInputRef"
          v-model="unlockPassword"
          type="password"
          show-password
          clearable
          size="large"
          placeholder="请输入密码"
          :disabled="unlocking"
          @keyup.enter="submitUnlock"
        />
        <p v-if="unlockError" class="unlock-error">{{ unlockError }}</p>
      </div>
      <template #footer>
        <div class="unlock-footer">
          <ElButton
            type="primary"
            size="large"
            class="unlock-confirm"
            :loading="unlocking"
            @click="submitUnlock"
          >
            确定
          </ElButton>
        </div>
      </template>
    </ElDialog>
  </div>
</template>

<style scoped>
.screensaver-root {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: #000;
  color: #fff;
}

.hint {
  position: fixed;
  right: 16px;
  bottom: 12px;
  margin: 0;
  font-size: 12px;
  letter-spacing: 0.04em;
  opacity: 0.35;
  pointer-events: none;
  z-index: 10;
}

.unlock-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.unlock-error {
  margin: 0;
  font-size: 13px;
  color: #ff6b6b;
}

.unlock-footer {
  display: flex;
  justify-content: center;
  width: 100%;
}

.unlock-confirm {
  min-width: 120px;
}
</style>

<style>
.screensaver-unlock-modal {
  background: rgba(0, 0, 0, 0.55) !important;
}

.screensaver-unlock-dialog.el-dialog {
  border-radius: 14px;
  overflow: hidden;
  background: #1a1a1e;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.55);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.screensaver-unlock-dialog .el-dialog__header {
  padding: 18px 20px 8px;
  margin: 0;
}

.screensaver-unlock-dialog .el-dialog__title {
  color: #f2f2f4;
  font-size: 16px;
  font-weight: 600;
}

.screensaver-unlock-dialog .el-dialog__body {
  padding: 12px 20px 8px;
  color: #e8e8ea;
}

.screensaver-unlock-dialog .el-dialog__footer {
  padding: 8px 20px 20px;
}

.screensaver-unlock-dialog .el-input__wrapper {
  background: #0f0f12;
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.12) inset;
}

.screensaver-unlock-dialog .el-input__inner {
  color: #f2f2f4;
}
</style>

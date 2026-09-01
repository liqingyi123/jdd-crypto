import { computed, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";

const MAX_KEYS = 4;
const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

export function formatShortcutDisplay(shortcut: string): string {
  return shortcut
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean)
    .join(" + ");
}

function mainKeyFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key)) {
    return null;
  }
  const code = event.code;
  if (code.startsWith("Key") && code.length === 4) {
    return code.slice(3);
  }
  if (code.startsWith("Digit") && code.length === 6) {
    return code.slice(5);
  }
  if (/^F\d{1,2}$/.test(code)) {
    return code;
  }
  if (code === "Space") {
    return "Space";
  }
  if (code.startsWith("Arrow")) {
    return code.slice(5);
  }
  if (event.key.length === 1) {
    return event.key.toUpperCase();
  }
  return event.key;
}

function partsFromEvent(event: KeyboardEvent): string[] {
  const parts: string[] = [];
  if (event.ctrlKey) {
    parts.push("Ctrl");
  }
  if (event.shiftKey) {
    parts.push("Shift");
  }
  if (event.altKey) {
    parts.push("Alt");
  }
  if (event.metaKey) {
    parts.push("Meta");
  }
  const main = mainKeyFromEvent(event);
  if (main) {
    parts.push(main);
  }
  return parts;
}

export function useShortcutRecorder(options?: {
  getCommand?: string;
  setCommand?: string;
  defaultShortcut?: string;
}) {
  const getCommand = options?.getCommand ?? "get_mouse_follow_shortcut";
  const setCommand = options?.setCommand ?? "set_mouse_follow_shortcut";
  const shortcut = shallowRef(options?.defaultShortcut ?? "Ctrl+Shift+G");
  const recording = shallowRef(false);
  const preview = shallowRef("");
  const errorMessage = shallowRef("");
  const buttonRef = shallowRef<HTMLElement | null>(null);

  const display = computed(() => formatShortcutDisplay(shortcut.value));
  const previewDisplay = computed(() =>
    preview.value ? formatShortcutDisplay(preview.value) : "请按下快捷键",
  );

  function onDocPointerDown(event: PointerEvent): void {
    const button = buttonRef.value;
    const target = event.target;
    if (button && target instanceof Node && button.contains(target)) {
      return;
    }
    void cancelRecording();
  }

  function bindOutsideListener(): void {
    document.addEventListener("pointerdown", onDocPointerDown, true);
  }

  function unbindOutsideListener(): void {
    document.removeEventListener("pointerdown", onDocPointerDown, true);
  }

  async function startRecording(): Promise<void> {
    if (recording.value) {
      return;
    }
    errorMessage.value = "";
    recording.value = true;
    preview.value = "";
    bindOutsideListener();
    await invoke("begin_shortcut_capture").catch(() => undefined);
  }

  async function cancelRecording(): Promise<void> {
    unbindOutsideListener();
    recording.value = false;
    preview.value = "";
    await invoke("end_shortcut_capture").catch(() => undefined);
  }

  async function commit(parts: string[]): Promise<void> {
    if (parts.length < 2 || parts.length > MAX_KEYS) {
      errorMessage.value = "快捷键需为 2–4 个键，且包含修饰键和主键";
      return;
    }
    const next = parts.join("+");
    try {
      const saved = await invoke<string>(setCommand, {
        shortcut: next,
      });
      unbindOutsideListener();
      shortcut.value = saved;
      recording.value = false;
      preview.value = "";
      errorMessage.value = "";
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : String(error);
    }
  }

  async function onRecordKey(event: KeyboardEvent): Promise<void> {
    if (!recording.value) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      await cancelRecording();
      return;
    }
    const parts = partsFromEvent(event);
    if (parts.length > MAX_KEYS) {
      return;
    }
    preview.value = parts.join("+");
    if (mainKeyFromEvent(event) && parts.length >= 2) {
      await commit(parts);
    }
  }

  onUnmounted(() => {
    unbindOutsideListener();
    if (recording.value) {
      void cancelRecording();
    }
  });

  return {
    shortcut,
    recording,
    errorMessage,
    display,
    previewDisplay,
    buttonRef,
    startRecording,
    cancelRecording,
    onRecordKey,
    loadShortcut: async () => {
      try {
        shortcut.value = await invoke<string>(getCommand);
      } catch {
        // browser preview
      }
    },
  };
}

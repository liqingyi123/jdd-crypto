import { onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  useThemeStore,
  type ResolvedTheme,
  type ThemePreference,
} from "@/stores/theme";

function readMediaTheme(): ResolvedTheme {
  if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
    return "dark";
  }
  return "light";
}

function asThemePreference(value: unknown): ThemePreference | null {
  if (value === "system" || value === "light" || value === "dark") {
    return value;
  }
  return null;
}

export function useSystemTheme() {
  const themeStore = useThemeStore();
  let media: MediaQueryList | null = null;
  let unlistenOsTheme: (() => void) | undefined;
  let unlistenPref: (() => void) | undefined;

  const onMediaChange = (event: MediaQueryListEvent) => {
    themeStore.setSystemTheme(event.matches ? "dark" : "light");
  };

  onMounted(async () => {
    themeStore.setSystemTheme(readMediaTheme());
    media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", onMediaChange);

    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      const current = await win.theme();
      if (current === "light" || current === "dark") {
        themeStore.setSystemTheme(current);
      }
      unlistenOsTheme = await win.onThemeChanged(({ payload }) => {
        if (payload === "light" || payload === "dark") {
          themeStore.setSystemTheme(payload);
        }
      });
    } catch {
      // browser preview: media query is enough
    }

    try {
      const saved = await invoke<string>("get_theme_pref");
      const preference = asThemePreference(saved);
      if (preference) {
        themeStore.setPreference(preference);
      }
      const { listen } = await import("@tauri-apps/api/event");
      unlistenPref = await listen<string>("app://theme-preference", (event) => {
        const preference = asThemePreference(event.payload);
        if (preference) {
          themeStore.setPreference(preference);
        }
      });
    } catch {
      // browser preview
    }

    themeStore.applyDocumentTheme();
  });

  onUnmounted(() => {
    media?.removeEventListener("change", onMediaChange);
    unlistenOsTheme?.();
    unlistenPref?.();
  });
}

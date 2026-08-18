import { onMounted, onUnmounted } from "vue";
import { useThemeStore, type ResolvedTheme } from "@/stores/theme";

function readMediaTheme(): ResolvedTheme {
  if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
    return "dark";
  }
  return "light";
}

export function useSystemTheme() {
  const themeStore = useThemeStore();
  let media: MediaQueryList | null = null;
  let unlistenTauri: (() => void) | undefined;

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
      unlistenTauri = await win.onThemeChanged(({ payload }) => {
        if (payload === "light" || payload === "dark") {
          themeStore.setSystemTheme(payload);
        }
      });
    } catch {
      // browser preview: media query is enough
    }

    themeStore.applyDocumentTheme();
  });

  onUnmounted(() => {
    media?.removeEventListener("change", onMediaChange);
    unlistenTauri?.();
  });
}

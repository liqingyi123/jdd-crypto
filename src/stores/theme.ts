import { defineStore } from "pinia";
import { computed, ref } from "vue";

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export const useThemeStore = defineStore("theme", () => {
  const preference = ref<ThemePreference>("system");
  const systemTheme = ref<ResolvedTheme>("light");

  const resolved = computed<ResolvedTheme>(() => {
    if (preference.value === "system") {
      return systemTheme.value;
    }
    return preference.value;
  });

  function setPreference(next: ThemePreference) {
    preference.value = next;
    applyDocumentTheme();
  }

  function setSystemTheme(next: ResolvedTheme) {
    systemTheme.value = next;
    applyDocumentTheme();
  }

  function applyDocumentTheme() {
    document.documentElement.dataset.theme = resolved.value;
    document.documentElement.classList.toggle("dark", resolved.value === "dark");
  }

  return {
    preference,
    systemTheme,
    resolved,
    setPreference,
    setSystemTheme,
    applyDocumentTheme,
  };
});

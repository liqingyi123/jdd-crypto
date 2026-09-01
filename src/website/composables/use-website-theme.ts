import { shallowRef } from "vue";

export type WebsiteThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "jdd-website-theme";

const preference = shallowRef<WebsiteThemePreference>("system");
let media: MediaQueryList | null = null;
let started = false;

function systemIsDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function resolveTheme(pref: WebsiteThemePreference): "light" | "dark" {
  if (pref === "system") {
    return systemIsDark() ? "dark" : "light";
  }
  return pref;
}

function applyTheme() {
  document.documentElement.dataset.theme = resolveTheme(preference.value);
}

function onSystemChange() {
  if (preference.value === "system") {
    applyTheme();
  }
}

export function useWebsiteTheme() {
  function initTheme() {
    if (started) {
      return;
    }
    started = true;
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "light" || saved === "dark" || saved === "system") {
      preference.value = saved;
    }
    applyTheme();
    media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", onSystemChange);
  }

  function setPreference(next: WebsiteThemePreference) {
    preference.value = next;
    localStorage.setItem(STORAGE_KEY, next);
    applyTheme();
  }

  return {
    preference,
    initTheme,
    setPreference,
  };
}

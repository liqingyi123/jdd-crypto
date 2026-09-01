<script setup lang="ts">
import { RouterLink, useRoute } from "vue-router";
import { useWebsiteTheme, type WebsiteThemePreference } from "../composables/use-website-theme";
import { useWebsiteDownload } from "../composables/use-website-download";

const route = useRoute();
const { preference, setPreference } = useWebsiteTheme();
const { href: downloadHref, label: downloadLabel, title: downloadTitle, ready: downloadReady } =
  useWebsiteDownload();
const base = import.meta.env.BASE_URL;

const nav = [
  { to: "/", label: "首页" },
  { to: "/features", label: "功能介绍" },
  { to: "/about", label: "关于" },
  { to: "/changelog", label: "更新日志" },
];

const themes: { id: WebsiteThemePreference; label: string }[] = [
  { id: "system", label: "跟随系统" },
  { id: "light", label: "浅色" },
  { id: "dark", label: "深色" },
];

function isActive(path: string) {
  return route.path === path;
}

function onDownloadClick(event: MouseEvent) {
  if (!downloadReady.value) {
    event.preventDefault();
  }
}
</script>

<template>
  <header class="header">
    <div class="container header-inner">
      <RouterLink class="brand" to="/">
        <img class="brand-logo" :src="`${base}app-icon.png`" width="32" height="32" alt="" />
        <span class="display-font">多多解密</span>
      </RouterLink>
      <nav class="nav" aria-label="主导航">
        <RouterLink
          v-for="item in nav"
          :key="item.to"
          :to="item.to"
          class="nav-link"
          :class="{ active: isActive(item.to) }"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
      <a
        class="download-btn"
        :class="{ disabled: !downloadReady }"
        :href="downloadHref"
        :title="downloadTitle"
        :aria-disabled="!downloadReady"
        @click="onDownloadClick"
      >
        {{ downloadLabel }}
      </a>
      <div class="theme-switch" role="group" aria-label="主题">
        <button
          v-for="item in themes"
          :key="item.id"
          type="button"
          class="theme-btn"
          :class="{ active: preference === item.id }"
          :aria-label="item.label"
          :title="item.label"
          @click="setPreference(item.id)"
        >
          <!-- system -->
          <svg
            v-if="item.id === 'system'"
            class="theme-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <rect
              x="3"
              y="4"
              width="18"
              height="12"
              rx="2"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
            />
            <path
              d="M8 20h8"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
            />
          </svg>
          <!-- light -->
          <svg
            v-else-if="item.id === 'light'"
            class="theme-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="4" fill="none" stroke="currentColor" stroke-width="1.8" />
            <path
              d="M12 3v2.2M12 18.8V21M3 12h2.2M18.8 12H21M5.6 5.6l1.6 1.6M16.8 16.8l1.6 1.6M5.6 18.4l1.6-1.6M16.8 7.2l1.6-1.6"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
            />
          </svg>
          <!-- dark -->
          <svg v-else class="theme-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M15.2 3.1a8.6 8.6 0 1 0 5.7 15.3A7 7 0 1 1 15.2 3.1z"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linejoin="round"
            />
          </svg>
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.header {
  position: sticky;
  top: 0;
  z-index: 20;
  backdrop-filter: blur(12px);
  background: color-mix(in srgb, var(--bg) 82%, transparent);
  border-bottom: 1px solid var(--border);
}

.header-inner {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 64px;
}

.brand {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-size: 1.25rem;
  font-weight: 800;
  color: var(--text);
  white-space: nowrap;
}

.brand-logo {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  object-fit: contain;
  flex-shrink: 0;
}

.nav {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 8px;
  flex: 1;
  min-width: 0;
}

.nav-link {
  padding: 8px 12px;
  border-radius: 8px;
  color: var(--text-muted);
  font-weight: 500;
  font-size: 0.95rem;
}

.nav-link:hover,
.nav-link.active {
  color: var(--text);
  background: var(--bg-muted);
}

.download-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px 14px;
  border-radius: 10px;
  background: var(--brand);
  color: #fff;
  font-size: 0.88rem;
  font-weight: 700;
  white-space: nowrap;
  flex-shrink: 0;
}

.download-btn:hover {
  filter: brightness(1.06);
}

.download-btn.disabled {
  opacity: 0.55;
  cursor: wait;
  filter: none;
}

.theme-switch {
  display: inline-flex;
  padding: 3px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  flex-shrink: 0;
}

.theme-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 30px;
  border: 0;
  background: transparent;
  color: var(--text-muted);
  border-radius: 7px;
  cursor: pointer;
  padding: 0;
}

.theme-btn.active {
  background: var(--brand-soft);
  color: var(--brand);
}

.theme-icon {
  width: 18px;
  height: 18px;
  display: block;
}

@media (max-width: 720px) {
  .header-inner {
    flex-wrap: wrap;
    padding-block: 10px;
    gap: 10px 12px;
  }

  .nav {
    order: 3;
    flex: 1 1 100%;
  }

  .download-btn {
    margin-left: auto;
    padding: 7px 12px;
    font-size: 0.82rem;
  }
}
</style>

<script setup lang="ts">
import { onMounted, reactive, watch } from "vue";
import { useChangelog } from "../composables/use-changelog";
import {
  detectPlatform,
  installerUrl,
  probeInstallerAvailable,
} from "../composables/use-website-download";

type ProbeState = "checking" | "available" | "missing";

const { entries, status, errorMessage, latestVersion, ensureLoaded } = useChangelog();
const platform = detectPlatform();
const platformLabel = platform === "macos" ? "macOS" : "Windows";
const availability = reactive<Record<string, ProbeState>>({});

function hrefFor(version: string) {
  return installerUrl(platform, version);
}

function isLatest(version: string) {
  return version === latestVersion.value;
}

async function probeVersions(versions: string[]) {
  const targets = versions.filter((version) => !isLatest(version));
  await Promise.all(
    targets.map(async (version) => {
      if (availability[version] === "available" || availability[version] === "missing") {
        return;
      }
      availability[version] = "checking";
      const ok = await probeInstallerAvailable(hrefFor(version));
      availability[version] = ok ? "available" : "missing";
    }),
  );
}

watch(
  entries,
  (list) => {
    if (list.length === 0) {
      return;
    }
    void probeVersions(list.map((item) => item.version));
  },
  { immediate: true },
);

onMounted(() => {
  void ensureLoaded().catch(() => {
    // errorMessage 已写入
  });
});
</script>

<template>
  <div class="page-section">
    <div class="container log">
      <p class="section-kicker">Changelog</p>
      <h1 class="section-title display-font">更新日志</h1>
      <p class="section-lead">按版本记录功能迭代与缺陷修复，便于对内发布与回溯。</p>

      <p v-if="status === 'loading'" class="status">正在加载更新日志…</p>
      <p v-else-if="status === 'error'" class="status error">{{ errorMessage }}</p>

      <ol v-else class="timeline">
        <li v-for="entry in entries" :key="entry.version" class="entry">
          <div class="version-row">
            <div class="version display-font">{{ entry.version }}</div>
            <span v-if="isLatest(entry.version)" class="dl-latest">当前最新版本</span>
            <a
              v-else-if="availability[entry.version] === 'available'"
              class="dl-btn"
              :href="hrefFor(entry.version)"
              :title="`下载 ${platformLabel} 安装包（v${entry.version}）`"
            >
              下载{{ platformLabel }}安装包
            </a>
            <span
              v-else-if="availability[entry.version] === 'missing'"
              class="dl-miss"
            >
              此版本未上线当前平台
            </span>
            <span v-else class="dl-checking">检测安装包…</span>
          </div>
          <ul>
            <li v-for="(note, index) in entry.notes" :key="index">{{ note }}</li>
          </ul>
        </li>
      </ol>
    </div>
  </div>
</template>

<style scoped>
.log {
  max-width: 760px;
}

.status {
  margin: 40px 0 0;
  color: var(--text-muted);
}

.status.error {
  color: var(--accent);
}

.timeline {
  list-style: none;
  margin: 40px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 28px;
  border-left: 2px solid var(--border);
}

.entry {
  position: relative;
  padding-left: 24px;
}

.entry::before {
  content: "";
  position: absolute;
  left: -7px;
  top: 8px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--brand);
  box-shadow: 0 0 0 4px var(--brand-soft);
}

.version-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px 14px;
  margin-bottom: 8px;
}

.version {
  font-size: 1.25rem;
  font-weight: 700;
}

.dl-btn {
  display: inline-flex;
  align-items: center;
  padding: 4px 12px;
  border-radius: 8px;
  background: var(--brand-soft);
  color: var(--brand);
  font-size: 0.82rem;
  font-weight: 700;
  white-space: nowrap;
}

.dl-btn:hover {
  filter: brightness(1.05);
}

.dl-latest {
  display: inline-flex;
  align-items: center;
  padding: 4px 12px;
  border-radius: 8px;
  background: var(--brand-soft);
  color: var(--brand);
  font-size: 0.82rem;
  font-weight: 700;
  white-space: nowrap;
}

.dl-miss,
.dl-checking {
  font-size: 0.82rem;
  color: var(--text-muted);
}

.dl-miss {
  color: var(--accent);
}

.entry ul {
  margin: 0;
  padding-left: 1.1rem;
  color: var(--text-muted);
  display: flex;
  flex-direction: column;
  gap: 6px;
}
</style>

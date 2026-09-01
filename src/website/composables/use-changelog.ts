import { computed, shallowRef } from "vue";

/** 与桌面端 `app_update.rs` 中 UPDATE_BASE 保持一致 */
export const UPDATE_BASE =
  "http://172.20.2.169:7101/appStore/Software/PC/developer/jdd-crypto";

export const CHANGELOG_URL = `${UPDATE_BASE}/${encodeURIComponent("更新日志.txt")}`;

export interface ChangelogEntry {
  version: string;
  notes: string[];
}

type LoadStatus = "idle" | "loading" | "ready" | "error";

const entries = shallowRef<ChangelogEntry[]>([]);
const status = shallowRef<LoadStatus>("idle");
const errorMessage = shallowRef("");
let loadPromise: Promise<ChangelogEntry[]> | null = null;

function parseVersion(version: string): [number, number, number] | null {
  const parts = version.split(".");
  if (parts.length !== 3) {
    return null;
  }
  const major = Number(parts[0]);
  const minor = Number(parts[1]);
  const patch = Number(parts[2]);
  if (![major, minor, patch].every((n) => Number.isInteger(n) && n >= 0)) {
    return null;
  }
  return [major, minor, patch];
}

function versionLt(a: string, b: string): boolean {
  const pa = parseVersion(a);
  const pb = parseVersion(b);
  if (!pa || !pb) {
    return false;
  }
  for (let i = 0; i < 3; i += 1) {
    if (pa[i] < pb[i]) {
      return true;
    }
    if (pa[i] > pb[i]) {
      return false;
    }
  }
  return false;
}

/** 解析与桌面端相同的 `【x.y.z】` 更新日志格式，按版本从高到低排序 */
export function parseChangelog(text: string): ChangelogEntry[] {
  const result: ChangelogEntry[] = [];
  const matches = [...text.matchAll(/【([^】]+)】/g)];

  for (let i = 0; i < matches.length; i += 1) {
    const match = matches[i];
    const version = match[1]?.trim() ?? "";
    if (!parseVersion(version)) {
      continue;
    }
    const start = (match.index ?? 0) + match[0].length;
    const end = i + 1 < matches.length ? (matches[i + 1].index ?? text.length) : text.length;
    const notes = text
      .slice(start, end)
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
    result.push({ version, notes });
  }

  result.sort((a, b) => {
    if (versionLt(a.version, b.version)) {
      return 1;
    }
    if (versionLt(b.version, a.version)) {
      return -1;
    }
    return 0;
  });

  return result;
}

export function useChangelog() {
  async function ensureLoaded(): Promise<ChangelogEntry[]> {
    if (status.value === "ready") {
      return entries.value;
    }
    if (loadPromise) {
      return loadPromise;
    }

    status.value = "loading";
    errorMessage.value = "";
    loadPromise = (async () => {
      try {
        const response = await fetch(CHANGELOG_URL);
        if (!response.ok) {
          throw new Error(`无法加载更新日志（${response.status}）`);
        }
        const text = await response.text();
        const parsed = parseChangelog(text);
        if (parsed.length === 0) {
          throw new Error("更新日志格式无效");
        }
        entries.value = parsed;
        status.value = "ready";
        return parsed;
      } catch (error) {
        status.value = "error";
        errorMessage.value =
          error instanceof Error ? error.message : "无法连接更新服务器";
        loadPromise = null;
        throw error;
      }
    })();

    return loadPromise;
  }

  const latestVersion = computed(() => entries.value[0]?.version ?? "");

  return {
    entries,
    status,
    errorMessage,
    latestVersion,
    ensureLoaded,
  };
}

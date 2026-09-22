import { computed, onMounted, shallowRef, watch } from "vue";
import { UPDATE_BASE, useChangelog } from "./use-changelog";

export type DownloadPlatform = "windows" | "macos";

/** Prefer current product name, then legacy rename. */
const PRODUCT_NAMES = ["多多工具箱", "多多解密"] as const;

export function detectPlatform(): DownloadPlatform {
  const ua = navigator.userAgent;
  if (/Macintosh|Mac OS X|iPhone|iPad|iPod/i.test(ua)) {
    return "macos";
  }
  return "windows";
}

export function installerFileNames(
  platform: DownloadPlatform,
  version: string,
): string[] {
  const suffix =
    platform === "macos" ? `${version}_universal.dmg` : `${version}_x64-setup.exe`;
  return PRODUCT_NAMES.map((name) => `${name}_${suffix}`);
}

/** Prefer 多多工具箱 naming; kept for callers that need a deterministic filename. */
export function installerFileName(platform: DownloadPlatform, version: string): string {
  return installerFileNames(platform, version)[0]!;
}

export function installerUrl(platform: DownloadPlatform, version: string): string {
  return `${UPDATE_BASE}/${encodeURIComponent(installerFileName(platform, version))}`;
}

export function installerCandidateUrls(
  platform: DownloadPlatform,
  version: string,
): string[] {
  return installerFileNames(platform, version).map(
    (file) => `${UPDATE_BASE}/${encodeURIComponent(file)}`,
  );
}

/** HEAD 探测安装包是否存在；不支持 HEAD 时回退 Range GET */
export async function probeInstallerAvailable(url: string): Promise<boolean> {
  try {
    const head = await fetch(url, { method: "HEAD" });
    if (head.ok) {
      return true;
    }
    if (head.status !== 405 && head.status !== 501) {
      return false;
    }
  } catch {
    // HEAD 被拒或 CORS 失败时再试 Range
  }

  try {
    const partial = await fetch(url, {
      method: "GET",
      headers: { Range: "bytes=0-0" },
    });
    return partial.ok || partial.status === 206;
  } catch {
    return false;
  }
}

/** Probe 多多工具箱 then 多多解密; return first available URL. */
export async function resolveInstallerUrl(
  platform: DownloadPlatform,
  version: string,
): Promise<string | null> {
  for (const url of installerCandidateUrls(platform, version)) {
    if (await probeInstallerAvailable(url)) {
      return url;
    }
  }
  return null;
}

/** 版本未就绪时避免跳到错误安装包 */
const CHANGELOG_PLACEHOLDER = "#";

export function useWebsiteDownload() {
  const platform = shallowRef<DownloadPlatform>(detectPlatform());
  const { latestVersion, ensureLoaded } = useChangelog();
  const resolvedHref = shallowRef(CHANGELOG_PLACEHOLDER);
  const resolved = shallowRef(false);
  let resolveToken = 0;

  async function resolveLatest() {
    const version = latestVersion.value;
    const token = ++resolveToken;
    resolved.value = false;
    resolvedHref.value = CHANGELOG_PLACEHOLDER;
    if (!version) {
      return;
    }
    const url = await resolveInstallerUrl(platform.value, version);
    if (token !== resolveToken) {
      return;
    }
    if (url) {
      resolvedHref.value = url;
      resolved.value = true;
    }
  }

  onMounted(() => {
    void ensureLoaded()
      .then(() => resolveLatest())
      .catch(() => {
        // 顶栏下载链接触发加载；失败时由更新日志页展示错误
      });
  });

  watch(latestVersion, () => {
    void resolveLatest();
  });

  const label = computed(() =>
    latestVersion.value ? `下载最新版（v${latestVersion.value}）` : "下载最新版",
  );

  const title = computed(() => {
    if (!latestVersion.value) {
      return "正在获取最新版本…";
    }
    if (!resolved.value) {
      return "正在检测安装包…";
    }
    return platform.value === "macos"
      ? `下载 macOS 安装包（v${latestVersion.value}）`
      : `下载 Windows 安装包（v${latestVersion.value}）`;
  });

  const href = computed(() => resolvedHref.value);
  const ready = computed(() => resolved.value);

  return {
    platform,
    version: latestVersion,
    label,
    title,
    href,
    ready,
  };
}

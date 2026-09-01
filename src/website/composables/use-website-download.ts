import { computed, onMounted, shallowRef } from "vue";
import { UPDATE_BASE, useChangelog } from "./use-changelog";

export type DownloadPlatform = "windows" | "macos";

export function detectPlatform(): DownloadPlatform {
  const ua = navigator.userAgent;
  if (/Macintosh|Mac OS X|iPhone|iPad|iPod/i.test(ua)) {
    return "macos";
  }
  return "windows";
}

export function installerFileName(platform: DownloadPlatform, version: string): string {
  if (platform === "macos") {
    return `多多解密_${version}_universal.dmg`;
  }
  return `多多解密_${version}_x64-setup.exe`;
}

export function installerUrl(platform: DownloadPlatform, version: string): string {
  return `${UPDATE_BASE}/${encodeURIComponent(installerFileName(platform, version))}`;
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

export function useWebsiteDownload() {
  const platform = shallowRef<DownloadPlatform>(detectPlatform());
  const { latestVersion, ensureLoaded } = useChangelog();

  onMounted(() => {
    void ensureLoaded().catch(() => {
      // 顶栏下载链接触发加载；失败时由更新日志页展示错误
    });
  });

  const label = computed(() =>
    latestVersion.value ? `下载最新版（v${latestVersion.value}）` : "下载最新版",
  );

  const title = computed(() => {
    if (!latestVersion.value) {
      return "正在获取最新版本…";
    }
    return platform.value === "macos"
      ? `下载 macOS 安装包（v${latestVersion.value}）`
      : `下载 Windows 安装包（v${latestVersion.value}）`;
  });

  const href = computed(() => {
    if (!latestVersion.value) {
      return CHANGELOG_PLACEHOLDER;
    }
    return installerUrl(platform.value, latestVersion.value);
  });

  const ready = computed(() => Boolean(latestVersion.value));

  return {
    platform,
    version: latestVersion,
    label,
    title,
    href,
    ready,
  };
}

/** 版本未就绪时避免跳到错误安装包 */
const CHANGELOG_PLACEHOLDER = "#";

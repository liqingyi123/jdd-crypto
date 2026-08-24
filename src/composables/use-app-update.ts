import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage } from "element-plus";

export type UpdatePhase = "prompt" | "downloading" | "ready";

export interface UpdateCheckResult {
  available: boolean;
  current_version: string;
  latest_version?: string | null;
  notes?: string | null;
  install_kind?: string | null;
}

export interface DownloadProgressPayload {
  downloaded: number;
  total?: number | null;
}

const visible = ref(false);
const phase = ref<UpdatePhase>("prompt");
const checking = ref(false);
const downloading = ref(false);
const installing = ref(false);
const currentVersion = ref("");
const latestVersion = ref("");
const notes = ref("");
const installKind = ref("nsis");
const installerPath = ref("");
const downloadedBytes = ref(0);
const totalBytes = ref<number | null>(null);

const progressPercent = computed(() => {
  if (totalBytes.value && totalBytes.value > 0) {
    return Math.min(100, Math.round((downloadedBytes.value / totalBytes.value) * 100));
  }
  return 0;
});

const downloadedLabel = computed(() => formatBytes(downloadedBytes.value));
const totalLabel = computed(() =>
  totalBytes.value ? formatBytes(totalBytes.value) : null,
);

const isDmgInstall = computed(() => installKind.value === "dmg");

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function resetDialogState() {
  phase.value = "prompt";
  downloading.value = false;
  installing.value = false;
  installerPath.value = "";
  downloadedBytes.value = 0;
  totalBytes.value = null;
}

function openPrompt(result: UpdateCheckResult) {
  latestVersion.value = result.latest_version ?? "";
  notes.value = result.notes ?? "";
  currentVersion.value = result.current_version;
  installKind.value = result.install_kind ?? "nsis";
  resetDialogState();
  visible.value = true;
}

export function applyUpdateResult(result: UpdateCheckResult) {
  openPrompt(result);
}

let progressUnlisten: (() => void) | undefined;

async function ensureProgressListener() {
  if (progressUnlisten) {
    return;
  }
  try {
    const { listen } = await import("@tauri-apps/api/event");
    progressUnlisten = await listen<DownloadProgressPayload>(
      "app://update-download-progress",
      (event) => {
        downloadedBytes.value = event.payload.downloaded;
        totalBytes.value = event.payload.total ?? null;
      },
    );
  } catch {
    // browser preview
  }
}

export function useAppUpdate() {
  async function checkUpdate(manual: boolean) {
    checking.value = true;
    try {
      const result = await invoke<UpdateCheckResult>("check_app_update", { manual });
      if (result.available) {
        openPrompt(result);
        return result;
      }
      if (manual) {
        ElMessage.success("当前已是最新版本");
      }
      return result;
    } catch (error) {
      if (manual) {
        ElMessage.error(
          error instanceof Error ? error.message : "无法连接更新服务器",
        );
      }
      return null;
    } finally {
      checking.value = false;
    }
  }

  async function startDownload() {
    if (!latestVersion.value || downloading.value) {
      return;
    }
    downloading.value = true;
    phase.value = "downloading";
    downloadedBytes.value = 0;
    totalBytes.value = null;
    await ensureProgressListener();
    try {
      installerPath.value = await invoke<string>("download_app_update", {
        version: latestVersion.value,
      });
      phase.value = "ready";
    } catch (error) {
      phase.value = "prompt";
      ElMessage.error(error instanceof Error ? error.message : "下载失败");
    } finally {
      downloading.value = false;
    }
  }

  async function installUpdate() {
    if (!installerPath.value || installing.value) {
      return;
    }
    installing.value = true;
    try {
      await invoke("install_app_update", { path: installerPath.value });
      if (isDmgInstall.value) {
        ElMessage.success("已打开安装镜像，请将应用拖入「应用程序」文件夹");
        installing.value = false;
      }
    } catch (error) {
      ElMessage.error(error instanceof Error ? error.message : "安装失败");
      installing.value = false;
    }
  }

  function dismissDialog() {
    if (downloading.value) {
      return;
    }
    visible.value = false;
  }

  return {
    visible,
    phase,
    checking,
    downloading,
    installing,
    currentVersion,
    latestVersion,
    notes,
    installKind,
    isDmgInstall,
    installerPath,
    downloadedBytes,
    totalBytes,
    progressPercent,
    downloadedLabel,
    totalLabel,
    checkUpdate,
    startDownload,
    installUpdate,
    dismissDialog,
  };
}

import { computed, ref } from "vue";

export interface EssentialsDownloadProgress {
  id: string;
  downloaded: number;
  total?: number | null;
}

const activeId = ref<string | null>(null);
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

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

let progressUnlisten: (() => void) | undefined;

async function ensureProgressListener() {
  if (progressUnlisten) {
    return;
  }
  try {
    const { listen } = await import("@tauri-apps/api/event");
    progressUnlisten = await listen<EssentialsDownloadProgress>(
      "app://essentials-download-progress",
      (event) => {
        const id = event.payload.id;
        if (activeId.value && activeId.value !== id) {
          return;
        }
        // Show progress only when backend actually starts downloading.
        if (!activeId.value) {
          activeId.value = id;
          downloadedBytes.value = 0;
          totalBytes.value = null;
        }
        downloadedBytes.value = event.payload.downloaded;
        totalBytes.value = event.payload.total ?? null;
      },
    );
  } catch {
    // browser preview
  }
}

export function useEssentialsDownload() {
  function begin(id: string) {
    activeId.value = id;
    downloadedBytes.value = 0;
    totalBytes.value = null;
    void ensureProgressListener();
  }

  function end() {
    activeId.value = null;
    downloadedBytes.value = 0;
    totalBytes.value = null;
  }

  return {
    activeId,
    downloadedBytes,
    totalBytes,
    progressPercent,
    downloadedLabel,
    totalLabel,
    begin,
    end,
    ensureProgressListener,
  };
}

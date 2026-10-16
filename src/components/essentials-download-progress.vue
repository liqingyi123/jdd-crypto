<script setup lang="ts">
import { useEssentialsDownload } from "@/composables/use-essentials-download";

defineProps<{
  hint: string;
}>();

const { progressPercent, downloadedLabel, totalLabel, totalBytes } =
  useEssentialsDownload();
</script>

<template>
  <div class="download-progress">
    <p class="progress-hint">{{ hint }}</p>
    <ElProgress
      :percentage="totalBytes ? progressPercent : undefined"
      :indeterminate="!totalBytes"
      :stroke-width="10"
    />
    <p class="progress-meta">
      已下载 {{ downloadedLabel }}
      <template v-if="totalLabel"> / {{ totalLabel }}</template>
    </p>
  </div>
</template>

<style scoped>
.download-progress {
  margin-top: 12px;
}

.progress-hint,
.progress-meta {
  margin: 0 0 8px;
  font-size: 13px;
  color: var(--text-muted);
}

.progress-meta {
  margin-top: 8px;
  margin-bottom: 0;
}
</style>

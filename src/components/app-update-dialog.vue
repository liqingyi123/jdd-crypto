<script setup lang="ts">
import { useAppUpdate } from "@/composables/use-app-update";

const {
  visible,
  phase,
  latestVersion,
  notes,
  downloading,
  installing,
  progressPercent,
  downloadedLabel,
  totalLabel,
  totalBytes,
  startDownload,
  installUpdate,
  dismissDialog,
} = useAppUpdate();
</script>

<template>
  <ElDialog
    v-model="visible"
    :title="`发现新版本 ${latestVersion}`"
    width="420px"
    :close-on-click-modal="!downloading"
    :close-on-press-escape="!downloading"
    :show-close="!downloading"
    @close="dismissDialog"
  >
    <div v-if="phase === 'prompt'" class="body">
      <p class="hint">当前有可用更新，更新内容如下：</p>
      <pre class="notes">{{ notes }}</pre>
    </div>

    <div v-else-if="phase === 'downloading'" class="body">
      <p class="hint">正在下载安装包…</p>
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

    <div v-else class="body">
      <p class="hint">安装包已下载完成，可立即安装新版本。</p>
    </div>

    <template #footer>
      <template v-if="phase === 'prompt'">
        <ElButton @click="dismissDialog">稍后</ElButton>
        <ElButton type="primary" @click="startDownload">下载更新</ElButton>
      </template>
      <template v-else-if="phase === 'downloading'">
        <ElButton disabled>下载中…</ElButton>
      </template>
      <template v-else>
        <ElButton @click="dismissDialog">稍后</ElButton>
        <ElButton type="primary" :loading="installing" @click="installUpdate">
          立即安装
        </ElButton>
      </template>
    </template>
  </ElDialog>
</template>

<style scoped>
.body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 13px;
}

.notes {
  margin: 0;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-muted);
  white-space: pre-wrap;
  font-family: inherit;
  font-size: 13px;
  line-height: 1.6;
  max-height: 220px;
  overflow: auto;
}

.progress-meta {
  margin: 0;
  font-size: 12px;
  color: var(--text-muted);
}
</style>

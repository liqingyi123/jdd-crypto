<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { useAppStore } from "@/stores/app";
import { useCryptoWorkspaceStore, type CryptoHistoryItem } from "@/stores/crypto-workspace";
import { AES_ENUM } from "@/utils/aes-enum";
import { runAes, type AesOpType } from "@/services/aes-ops";
import { prettierFormat } from "@/utils/prettier-format";
import MonacoEditor from "@/components/monaco-editor.vue";

const appStore = useAppStore();
const workspace = useCryptoWorkspaceStore();
const { pendingCrypto } = storeToRefs(appStore);
const { aesCode, customKey, customIv, history } = storeToRefs(workspace);

const inputText = ref("");
const outputText = ref("");
const errorMessage = ref("");
const historyVisible = ref(false);

const isCustom = computed(() => aesCode.value === "custom");

const typeLabel: Record<AesOpType, string> = {
  encrypt: "加密",
  decrypt: "解密",
  tokv: "转KV",
};

const typeTagType: Record<AesOpType, "success" | "primary" | "warning"> = {
  encrypt: "success",
  decrypt: "primary",
  tokv: "warning",
};

function formatAt(at: number): string {
  const d = new Date(at);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

function truncate(text: string, max = 80): string {
  const oneLine = text.replace(/\s+/g, " ").trim();
  if (oneLine.length <= max) {
    return oneLine;
  }
  return `${oneLine.slice(0, max)}…`;
}

function formatAesConfig(item: CryptoHistoryItem): string {
  const preset = AES_ENUM.find((entry) => entry.code === item.aesCode);
  const label = preset?.who ?? item.aesCode;

  if (item.aesCode === "custom" || item.aesCode === "auto") {
    const key = item.key || "—";
    const iv = item.iv || "—";
    return `${label} · Key: ${truncate(key, 32)} · IV: ${truncate(iv, 16)}`;
  }

  return label;
}

function removeHistoryItem(event: MouseEvent, index: number) {
  event.stopPropagation();
  event.preventDefault();
  workspace.removeHistoryAt(index);
}

function clearInput() {
  inputText.value = "";
  errorMessage.value = "";
}

async function run(type: AesOpType) {
  errorMessage.value = "";
  const result = runAes({
    type,
    text: inputText.value,
    aesCode: aesCode.value,
    customKey: customKey.value,
    customIv: customIv.value,
  });

  if (result.code === "empty") {
    errorMessage.value = "请输入待处理文本，并确认 AES key / iv 已填写";
    outputText.value = "";
    return;
  }

  if (result.code === "error") {
    outputText.value = result.content;
    errorMessage.value = result.content;
    return;
  }

  const display = await prettierFormat(result.content);
  outputText.value = display;

  const usedCode = result.usedCode ?? aesCode.value;
  const usedKey = result.usedKey ?? "";
  const usedIv = result.usedIv ?? "";

  if (aesCode.value === "custom" || usedCode === "custom") {
    workspace.rememberCustom(usedKey, usedIv);
  }

  workspace.pushHistory({
    type,
    text: inputText.value,
    aesCode: usedCode,
    key: usedKey,
    iv: usedIv,
    result: display,
  });
}

async function applyHistory(id: string) {
  const item = history.value.find((entry) => entry.id === id);
  if (!item) {
    return;
  }
  historyVisible.value = false;

  const known = AES_ENUM.some((preset) => preset.code === item.aesCode);
  if (item.aesCode === "custom" || !known) {
    workspace.setAesCode("custom");
    workspace.setCustomKey(item.key);
    workspace.setCustomIv(item.iv);
  } else {
    workspace.setAesCode(item.aesCode);
  }

  inputText.value = item.text;
  await run(item.type);
}

async function applyPending() {
  const payload = pendingCrypto.value;
  if (!payload) {
    return;
  }
  inputText.value = payload.text;
  const mode = payload.mode === "encrypt" ? "encrypt" : "decrypt";
  appStore.setPendingCrypto(null);
  await run(mode);
}

onMounted(() => {
  void applyPending();
});

watch(pendingCrypto, () => {
  void applyPending();
});
</script>

<template>
  <div class="page">
    <div class="toolbar">
      <label class="field">
        AES 配置
        <ElSelect
          :model-value="aesCode"
          class="select"
          @update:model-value="workspace.setAesCode"
        >
          <ElOption
            v-for="item in AES_ENUM"
            :key="item.code"
            :label="item.who"
            :value="item.code"
          />
        </ElSelect>
      </label>

      <template v-if="isCustom">
        <label class="field grow">
          Key
          <ElInput
            :model-value="customKey"
            placeholder="AES Key"
            @update:model-value="workspace.setCustomKey"
          />
        </label>
        <label class="field grow">
          IV
          <ElInput
            :model-value="customIv"
            placeholder="AES IV"
            @update:model-value="workspace.setCustomIv"
          />
        </label>
      </template>

      <ElButton class="history-btn" @click="historyVisible = true">
        操作历史
      </ElButton>
    </div>

    <div class="actions">
      <ElButton type="success" @click="run('encrypt')">加密</ElButton>
      <ElButton type="primary" @click="run('decrypt')">解密</ElButton>
      <ElButton type="warning" @click="run('tokv')">转 KV</ElButton>
    </div>

    <div class="editors">
      <div class="pane">
        <div class="pane-header">
          <span class="pane-label">待处理文本</span>
        </div>
        <MonacoEditor
          v-model="inputText"
          language="javascript"
          clear-on-double-click
          @clear="clearInput"
        />
      </div>
      <div class="pane">
        <div class="pane-header">
          <span class="pane-label">输出结果</span>
        </div>
        <MonacoEditor
          v-model="outputText"
          language="javascript"
          read-only
          :folding="true"
        />
      </div>
    </div>

    <p v-if="errorMessage" class="error">{{ errorMessage }}</p>

    <ElDialog
      v-model="historyVisible"
      title="操作历史"
      width="720px"
      destroy-on-close
    >
      <div v-if="history.length === 0" class="empty">暂无记录</div>
      <ul v-else class="history-list">
        <li v-for="(item, index) in history" :key="item.id" class="history-item">
          <div class="history-body" @click="applyHistory(item.id)">
            <div class="history-meta">
              <ElTag :type="typeTagType[item.type]" size="small">
                {{ typeLabel[item.type] }}
              </ElTag>
              <span class="history-time">{{ formatAt(item.at) }}</span>
              <button
                type="button"
                class="history-delete"
                @click="removeHistoryItem($event, index)"
              >
                删除
              </button>
            </div>
            <div class="history-aes" :title="formatAesConfig(item)">
              AES: {{ formatAesConfig(item) }}
            </div>
            <div class="history-text" :title="item.text">
              {{ truncate(item.text) || "（空）" }}
            </div>
          </div>
        </li>
      </ul>
      <template #footer>
        <ElButton
          :disabled="history.length === 0"
          @click="workspace.clearHistory()"
        >
          清空历史
        </ElButton>
        <ElButton type="primary" @click="historyVisible = false">关闭</ElButton>
      </template>
    </ElDialog>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: end;
  flex-shrink: 0;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  color: var(--text-muted);
  font-size: 13px;
  min-width: 160px;
}

.field.grow {
  flex: 1;
  min-width: 180px;
}

.select {
  width: 200px;
}

.history-btn {
  margin-left: auto;
}

.actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.editors {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

.pane {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  min-height: 0;
  height: 100%;
}

.pane-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.pane-label {
  font-size: 13px;
  color: var(--text-muted);
}

.pane :deep(.monaco-host) {
  flex: 1;
  min-height: 0;
  height: auto;
}

.pane :deep(.monaco-editor-root) {
  min-height: 0;
  height: 100%;
}

.error {
  margin: 0;
  color: var(--danger);
  font-size: 13px;
  white-space: pre-wrap;
  flex-shrink: 0;
}

.empty {
  color: var(--text-muted);
  text-align: center;
  padding: 24px 0;
}

.history-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 420px;
  overflow: auto;
}

.history-item {
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  margin-bottom: 8px;
  background: var(--bg-elevated);
}

.history-body {
  min-width: 0;
  cursor: pointer;
}

.history-item:hover {
  border-color: var(--brand);
  background: var(--brand-soft);
}

.history-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.history-delete {
  margin-left: auto;
  flex-shrink: 0;
  border: none;
  background: transparent;
  color: var(--danger);
  font-size: 12px;
  line-height: 1;
  padding: 4px 6px;
  border-radius: 4px;
  cursor: pointer;
}

.history-delete:hover {
  background: color-mix(in srgb, var(--danger) 12%, transparent);
}

.history-time {
  font-size: 12px;
  color: var(--text-muted);
}

.history-aes {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 4px;
  word-break: break-all;
}

.history-text {
  font-size: 13px;
  color: var(--text);
  word-break: break-all;
}
</style>

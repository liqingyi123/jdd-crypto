<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { useAppStore } from "@/stores/app";
import { usePluginsStore } from "@/stores/plugins";
import { transform } from "@/services/crypto";

const appStore = useAppStore();
const pluginsStore = usePluginsStore();
const { pendingCrypto } = storeToRefs(appStore);
const { cryptoOptions } = storeToRefs(pluginsStore);

const inputText = ref("");
const outputText = ref("");
const algorithm = ref("AES-256-GCM");
const errorMessage = ref("");
const mode = ref<"encrypt" | "decrypt">("encrypt");

const algorithms = ["AES-256-GCM", "AES-256-CBC", "SM4"];

onMounted(() => {
  applyPending();
});

watch(pendingCrypto, () => {
  applyPending();
});

function applyPending() {
  const payload = pendingCrypto.value;
  if (!payload) {
    return;
  }
  inputText.value = payload.text;
  mode.value = payload.mode === "decrypt" ? "decrypt" : "encrypt";
  appStore.setPendingCrypto(null);
}

async function run() {
  errorMessage.value = "";
  outputText.value = "";
  try {
    outputText.value = await transform({
      mode: mode.value,
      algorithm: algorithm.value,
      plaintext: mode.value === "encrypt" ? inputText.value : undefined,
      ciphertext: mode.value === "decrypt" ? inputText.value : undefined,
    });
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}
</script>

<template>
  <div class="page">
    <p class="hint">算法实现仍为 Rust stub，当前用于打通主界面工作流。</p>
    <div class="toolbar">
      <label>
        模式
        <ElSelect v-model="mode" class="select">
          <ElOption label="加密" value="encrypt" />
          <ElOption label="解密" value="decrypt" />
        </ElSelect>
      </label>
      <label>
        算法
        <ElSelect v-model="algorithm" class="select">
          <ElOption v-for="item in algorithms" :key="item" :label="item" :value="item" />
          <ElOption
            v-for="item in cryptoOptions"
            :key="item.id"
            :label="item.label"
            :value="item.algorithm"
          />
        </ElSelect>
      </label>
      <ElButton type="primary" class="run" @click="run">执行</ElButton>
    </div>
    <div class="grid">
      <label>
        输入
        <ElInput
          v-model="inputText"
          type="textarea"
          :rows="12"
          placeholder="明文或密文"
        />
      </label>
      <label>
        输出
        <ElInput
          v-model="outputText"
          type="textarea"
          :rows="12"
          readonly
          placeholder="处理结果"
        />
      </label>
    </div>
    <p v-if="errorMessage" class="error">{{ errorMessage }}</p>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.hint,
.error {
  margin: 0;
  color: var(--text-muted);
}

.error {
  color: var(--danger);
}

.toolbar,
.grid {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.toolbar label,
.grid label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  color: var(--text-muted);
  font-size: 13px;
}

.select {
  width: 180px;
}

.run {
  align-self: end;
}

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
}
</style>

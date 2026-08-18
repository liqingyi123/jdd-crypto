<script setup lang="ts">
import { ref } from "vue";

const content = ref("");
const sent = ref(false);

function submit() {
  sent.value = true;
}
</script>

<template>
  <div class="page">
    <p>反馈通道为占位实现，后续可接入邮件或远程接口。</p>
    <textarea v-model="content" placeholder="描述你遇到的问题或建议" />
    <button type="button" :disabled="!content.trim()" @click="submit">提交反馈</button>
    <p v-if="sent" class="ok">已记录到本地（尚未发送）。</p>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-width: 640px;
}

textarea {
  min-height: 180px;
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 12px;
  background: var(--bg-elevated);
  color: var(--text);
}

button {
  width: fit-content;
  border: 0;
  border-radius: 10px;
  padding: 8px 14px;
  background: var(--brand);
  color: #fff;
}

button:disabled {
  opacity: 0.5;
}

.ok {
  color: var(--text-muted);
}
</style>

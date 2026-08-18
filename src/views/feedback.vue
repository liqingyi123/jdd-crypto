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
    <ElInput
      v-model="content"
      type="textarea"
      :rows="8"
      placeholder="描述你遇到的问题或建议"
    />
    <ElButton type="primary" class="submit" :disabled="!content.trim()" @click="submit">
      提交反馈
    </ElButton>
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

.submit {
  width: fit-content;
}

.ok {
  color: var(--text-muted);
}
</style>

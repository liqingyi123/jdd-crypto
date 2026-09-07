<script setup lang="ts">
import { ref } from "vue";
import { ElMessage } from "element-plus";

const content = ref("");

function submit() {
  ElMessage.info("反馈通道尚未接入，内容不会保存或发送，请通过关于页联系方式反馈。");
}
</script>

<template>
  <div class="page">
    <p>
      反馈通道为占位页面，当前不会保存或上报内容。如需联系，请打开「关于」查看联系方式。
    </p>
    <ElInput
      v-model="content"
      type="textarea"
      :rows="8"
      placeholder="描述你遇到的问题或建议（仅本地预览，提交无效）"
    />
    <ElButton type="primary" class="submit" :disabled="!content.trim()" @click="submit">
      提交反馈
    </ElButton>
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
</style>

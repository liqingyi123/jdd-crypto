<script setup lang="ts">
import zhCn from "element-plus/es/locale/lang/zh-cn";
import BadgeApp from "@/windows/badge-app.vue";
import MainApp from "@/windows/main-app.vue";
import FeatureApp from "@/windows/feature-app.vue";

defineProps<{
  windowLabel: string;
}>();
</script>

<template>
  <ElConfigProvider :locale="zhCn">
    <BadgeApp v-if="windowLabel === 'badge'" />
    <MainApp v-else-if="windowLabel === 'main'" />
    <FeatureApp v-else :window-label="windowLabel" />
  </ElConfigProvider>
</template>

<script setup lang="ts">
import { defineAsyncComponent } from "vue";

defineProps<{
  windowLabel: string;
}>();

const BadgeApp = defineAsyncComponent(() => import("@/windows/badge-app.vue"));
const MainApp = defineAsyncComponent(() => import("@/windows/main-app.vue"));
const FeatureApp = defineAsyncComponent(() => import("@/windows/feature-app.vue"));
</script>

<template>
  <BadgeApp v-if="windowLabel === 'badge'" />
  <MainApp v-else-if="windowLabel === 'main'" />
  <FeatureApp v-else :window-label="windowLabel" />
</template>

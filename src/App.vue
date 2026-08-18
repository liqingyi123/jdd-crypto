<script setup lang="ts">
import BadgeApp from "@/windows/badge-app.vue";
import MainApp from "@/windows/main-app.vue";
import FeatureApp from "@/windows/feature-app.vue";

defineProps<{
  windowLabel: string;
}>();
</script>

<template>
  <BadgeApp v-if="windowLabel === 'badge'" />
  <MainApp v-else-if="windowLabel === 'main'" />
  <FeatureApp v-else :window-label="windowLabel" />
</template>

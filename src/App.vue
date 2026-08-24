<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import MouseTrailApp from "@/windows/mouse-trail-app.vue";

const props = defineProps<{
  windowLabel: string;
}>();

const BadgeApp = defineAsyncComponent(() => import("@/windows/badge-app.vue"));
const ClipboardPromptApp = defineAsyncComponent(
  () => import("@/windows/clipboard-prompt-app.vue"),
);
const MainApp = defineAsyncComponent(() => import("@/windows/main-app.vue"));
const FeatureApp = defineAsyncComponent(() => import("@/windows/feature-app.vue"));
const AppUpdateHost = defineAsyncComponent(
  () => import("@/components/app-update-host.vue"),
);

const isMouseTrail = computed(() => props.windowLabel.startsWith("mouse-trail"));
</script>

<template>
  <BadgeApp v-if="windowLabel === 'badge'" />
  <ClipboardPromptApp v-else-if="windowLabel === 'clipboard-prompt'" />
  <MainApp v-else-if="windowLabel === 'main'" />
  <MouseTrailApp v-else-if="isMouseTrail" />
  <FeatureApp v-else :window-label="windowLabel" />
  <AppUpdateHost v-if="windowLabel === 'about'" />
</template>

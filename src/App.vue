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
const CryptoBubbleApp = defineAsyncComponent(
  () => import("@/windows/crypto-bubble-app.vue"),
);
const CompareTipApp = defineAsyncComponent(
  () => import("@/windows/compare-tip-app.vue"),
);
const CompareBubbleApp = defineAsyncComponent(
  () => import("@/windows/compare-bubble-app.vue"),
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
  <CryptoBubbleApp v-else-if="windowLabel === 'crypto-bubble'" />
  <CompareTipApp v-else-if="windowLabel === 'compare-tip'" />
  <CompareBubbleApp v-else-if="windowLabel === 'compare-bubble'" />
  <MainApp v-else-if="windowLabel === 'main'" />
  <MouseTrailApp v-else-if="isMouseTrail" />
  <FeatureApp v-else :window-label="windowLabel" />
  <AppUpdateHost v-if="windowLabel === 'about'" />
</template>

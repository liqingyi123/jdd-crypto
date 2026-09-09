<script setup lang="ts">
import { computed, type Component } from "vue";
import { useSystemTheme } from "@/composables/use-system-theme";
import Settings from "@/views/settings.vue";
import About from "@/views/about.vue";
import Hosts from "@/views/hosts.vue";
import zhCn from "element-plus/es/locale/lang/zh-cn";

useSystemTheme();

const props = defineProps<{
  windowLabel: string;
}>();

const pages: Record<string, { title: string; component: Component }> = {
  settings: { title: "功能设置", component: Settings },
  about: { title: "关于", component: About },
  hosts: { title: "Host管理", component: Hosts },
};

const page = computed(() => pages[props.windowLabel]);
const fillContent = computed(
  () => props.windowLabel === "hosts" || props.windowLabel === "settings",
);
</script>

<template>
  <ElConfigProvider :locale="zhCn">
    <div v-if="page" class="shell" :class="{ 'shell-fill': fillContent }">
      <section
        class="content"
        :class="{
          'content-fill': fillContent,
          'content-settings': props.windowLabel === 'settings',
        }"
      >
        <component :is="page.component" />
      </section>
    </div>
  </ElConfigProvider>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.content {
  flex: 1;
  padding: 16px 24px 24px;
  overflow: auto;
}

.content-settings {
  padding: 4px 24px 16px;
}

.shell-fill .content-fill {
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.shell-fill .content-fill:not(.content-settings) {
  padding: 12px 16px 16px;
}
</style>

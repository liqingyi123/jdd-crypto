<script setup lang="ts">
import { computed, type Component } from "vue";
import { useSystemTheme } from "@/composables/use-system-theme";
import Settings from "@/views/settings.vue";
import Feedback from "@/views/feedback.vue";
import Plugins from "@/views/plugins.vue";
import About from "@/views/about.vue";
import Hosts from "@/views/hosts.vue";
import zhCn from "element-plus/es/locale/lang/zh-cn";

useSystemTheme();

const props = defineProps<{
  windowLabel: string;
}>();

const pages: Record<string, { title: string; component: Component }> = {
  settings: { title: "功能设置", component: Settings },
  feedback: { title: "意见反馈", component: Feedback },
  plugins: { title: "插件管理", component: Plugins },
  about: { title: "关于", component: About },
  hosts: { title: "Host管理", component: Hosts },
};

const page = computed(() => pages[props.windowLabel]);
const fillContent = computed(() => props.windowLabel === "hosts");
</script>

<template>
  <ElConfigProvider :locale="zhCn">
    <div v-if="page" class="shell" :class="{ 'shell-fill': fillContent }">
      <header class="header">
        <h1 class="title">{{ page.title }}</h1>
      </header>
      <section class="content" :class="{ 'content-fill': fillContent }">
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

.header {
  padding: 18px 24px 8px;
}

.title {
  margin: 0;
  font-size: 20px;
}

.content {
  flex: 1;
  padding: 8px 24px 24px;
  overflow: auto;
}

.shell-fill .content-fill {
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
</style>

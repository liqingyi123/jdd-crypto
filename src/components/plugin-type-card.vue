<script setup lang="ts">
import type { PluginSlot } from "@/constants/plugin-slots";

defineProps<{
  title: string;
  description: string;
  slot: PluginSlot;
  comingSoon: boolean;
  importing: boolean;
  resetting: boolean;
}>();

const emit = defineEmits<{
  toggle: [enabled: boolean];
  import: [];
  reset: [];
}>();

function displayName(slot: PluginSlot): string {
  return slot.current?.name ?? "未设置";
}
</script>

<template>
  <section>
    <div class="section-head">
      <h2>{{ title }}</h2>
      <ElSwitch
        :model-value="slot.enabled"
        @change="(value) => emit('toggle', Boolean(value))"
      />
    </div>
    <p>{{ description }}</p>
    <div class="row">
      <span class="current">当前：{{ displayName(slot) }}</span>
      <div class="actions">
        <ElButton :loading="importing" @click="emit('import')">导入插件</ElButton>
        <ElButton :loading="resetting" @click="emit('reset')">恢复默认</ElButton>
      </div>
    </div>
    <p v-if="comingSoon" class="hint">开发中，敬请期待</p>
  </section>
</template>

<style scoped>
section {
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-elevated);
}

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

h2 {
  margin: 0;
  font-size: 16px;
}

p {
  margin: 8px 0 0;
  color: var(--text-muted);
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 12px;
}

.current {
  color: var(--text);
  font-size: 13px;
}

.actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.hint {
  color: var(--brand);
}
</style>

<script setup lang="ts">
import { TRAIL_EFFECT_OPTIONS } from "../data/features";
import { useGlobalTrail } from "../composables/use-global-trail";
import type { MouseTrailEffect } from "@/effects/mouse-trail-types";

const { effect, color, colorable, setEffect, setColor } = useGlobalTrail();

function onColorInput(event: Event) {
  const target = event.target as HTMLInputElement;
  setColor(target.value);
}

function onSelectEffect(next: MouseTrailEffect) {
  setEffect(next);
}
</script>

<template>
  <div class="demo">
    <div class="toolbar">
      <div class="effects" role="group" aria-label="特效">
        <button
          v-for="item in TRAIL_EFFECT_OPTIONS"
          :key="item.id"
          type="button"
          class="chip"
          :class="{ active: effect === item.id }"
          @click="onSelectEffect(item.id)"
        >
          {{ item.label }}
        </button>
      </div>
      <label v-if="colorable" class="color-row">
        <span>颜色</span>
        <input :value="color" type="color" @input="onColorInput" />
      </label>
    </div>
    <p class="hint">拖尾已在全站生效，移动鼠标即可体验；在此切换特效与颜色。</p>
  </div>
</template>

<style scoped>
.demo {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 20px;
  border-radius: 16px;
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  box-shadow: var(--shadow);
}

.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.effects {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.chip {
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-muted);
  border-radius: 999px;
  padding: 8px 14px;
  cursor: pointer;
  font-weight: 600;
  font-size: 0.88rem;
}

.chip.active {
  border-color: var(--brand);
  background: var(--brand-soft);
  color: var(--brand);
}

.color-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 0.9rem;
}

.color-row input {
  width: 40px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: transparent;
  cursor: pointer;
}

.hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.95rem;
}
</style>

<script setup lang="ts">
defineProps<{
  text: string;
}>();
</script>

<template>
  <div v-if="text" class="overlay-toast" aria-live="polite">{{ text }}</div>
</template>

<style scoped>
.overlay-toast {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  z-index: 10;
  max-width: none;
  pointer-events: none;
  padding: 12px 20px;
  border-radius: 12px;
  background: rgba(20, 24, 32, 0.78);
  color: #fff;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.02em;
  line-height: 1.4;
  text-align: center;
  white-space: nowrap;
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.28);
}
</style>

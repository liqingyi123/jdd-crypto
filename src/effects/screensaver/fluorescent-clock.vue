<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import "./fluorescent-clock.css";

const timeRef = ref<HTMLElement | null>(null);
const hours = ref("00");
const minutes = ref("00");
const seconds = ref("00");

let timer: ReturnType<typeof setInterval> | null = null;
let rafId = 0;
let targetX = 0;
let targetY = 0;
let currentX = 0;
let currentY = 0;

function pad(value: number) {
  return value < 10 ? `0${value}` : String(value);
}

function tick() {
  const now = new Date();
  hours.value = pad(now.getHours());
  minutes.value = pad(now.getMinutes());
  seconds.value = pad(now.getSeconds());
}

function onMouseMove(event: MouseEvent) {
  const w = window.innerWidth || 1;
  const h = window.innerHeight || 1;
  targetX = (w / 2 - event.clientX) / w;
  targetY = (h / 2 - event.clientY) / h;
}

function lerpFrame() {
  currentX += (targetX - currentX) * 0.2;
  currentY += (targetY - currentY) * 0.2;
  const el = timeRef.value;
  if (el) {
    el.style.setProperty("--mouse-x", currentX.toFixed(4));
    el.style.setProperty("--mouse-y", currentY.toFixed(4));
  }
  rafId = window.requestAnimationFrame(lerpFrame);
}

onMounted(() => {
  tick();
  timer = setInterval(tick, 1000);
  window.addEventListener("mousemove", onMouseMove);
  rafId = window.requestAnimationFrame(lerpFrame);
});

onUnmounted(() => {
  if (timer !== null) {
    clearInterval(timer);
    timer = null;
  }
  window.removeEventListener("mousemove", onMouseMove);
  window.cancelAnimationFrame(rafId);
});
</script>

<template>
  <div class="fluorescent-clock">
    <div
      ref="timeRef"
      class="time"
      :data-hours="hours"
      :data-minutes="minutes"
      :data-seconds="seconds"
    >
      <div v-for="n in 6" :key="n" class="digit">
        <div v-for="line in 7" :key="line" class="line" />
      </div>
    </div>
  </div>
</template>

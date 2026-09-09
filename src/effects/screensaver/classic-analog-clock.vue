<script setup lang="ts">
/**
 * Classic dashed-tick analog clock migrated from jiaoben6996.
 */
import { computed, onMounted, onUnmounted, shallowRef } from "vue";
import "./clock-chroma.css";

function isInt(n: number) {
  return Math.floor(n) === n;
}

function rotate(x: number, y: number, angle: number) {
  const rad = (angle === 0 ? 270 : angle) * (Math.PI / 180);
  const A = Math.atan2(y, x) + rad;
  const R = Math.sqrt(x * x + y * y);
  return {
    x: Math.cos(A) * R,
    y: Math.sin(A) * R,
  };
}

const marks = Array.from({ length: 60 }, (_, i) => {
  const n = (i / 5) % 12;
  const label = n === 0 ? 12 : n;
  return {
    deg: i * 6,
    bold: isInt(label),
  };
});

const numberLabels = (() => {
  const angle = 30;
  let x = 110;
  let y = -190;
  const list: Array<{ n: number; x: number; y: number }> = [{ n: 1, x, y }];
  for (let i = 2; i <= 12; i++) {
    const pos = rotate(x, y, angle);
    x = pos.x;
    y = pos.y;
    list.push({ n: i, x, y });
  }
  return list;
})();

const hourDeg = shallowRef(0);
const minuteDeg = shallowRef(0);
const secondDeg = shallowRef(0);
const clockText = shallowRef("");

let rafId = 0;
let running = false;

function pad(n: number) {
  return n < 10 ? `0${n}` : String(n);
}

function formatClockText(t: Date) {
  const ymd = `${t.getFullYear()}年${t.getMonth() + 1}月${t.getDate()}日`;
  const hours24 = t.getHours();
  const period = hours24 < 12 ? "上午" : "下午";
  const hours12 = hours24 % 12 || 12;
  const hms = `${pad(hours12)}:${pad(t.getMinutes())}:${pad(t.getSeconds())}`;
  const week = `星期${"日一二三四五六"[t.getDay()]}`;
  return `${ymd} ${period}${hms} ${week}`;
}

function tick() {
  if (!running) {
    return;
  }
  const t = new Date();
  hourDeg.value = t.getHours() * 30 + (t.getMinutes() / 60) * 30;
  minuteDeg.value = t.getMinutes() * 6 + (t.getSeconds() / 60) * 6;
  secondDeg.value = t.getSeconds() * 6 + (t.getMilliseconds() / 1000) * 6;
  clockText.value = formatClockText(t);
  rafId = window.requestAnimationFrame(tick);
}

const hourStyle = computed(() => ({ transform: `rotate(${hourDeg.value}deg)` }));
const minuteStyle = computed(() => ({ transform: `rotate(${minuteDeg.value}deg)` }));
const secondStyle = computed(() => ({ transform: `rotate(${secondDeg.value}deg)` }));

onMounted(() => {
  running = true;
  rafId = window.requestAnimationFrame(tick);
});

onUnmounted(() => {
  running = false;
  window.cancelAnimationFrame(rafId);
});
</script>

<template>
  <div class="analog-root clock-chroma">
    <div class="clock">
      <ul class="mark">
        <li
          v-for="(item, index) in marks"
          :key="index"
          :class="{ bold: item.bold }"
          :style="{ transform: `translateY(250px) rotate(${item.deg}deg)` }"
        />
      </ul>
      <div
        v-for="item in numberLabels"
        :key="item.n"
        class="numbers"
        :style="{ transform: `translate(${item.x}px, ${item.y}px)` }"
      >
        {{ item.n }}
      </div>
      <div class="time">{{ clockText }}</div>
      <div class="hour-hand" :style="hourStyle" />
      <div class="minute-hand" :style="minuteStyle" />
      <div class="second-hand" :style="secondStyle" />
      <div class="center" />
    </div>
  </div>
</template>

<style scoped>
.analog-root {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  z-index: 5;
}

.clock {
  width: 500px;
  height: 500px;
  position: relative;
  margin: auto;
  transform: scale(calc(min(720px, 94vmin) / 500px));
  transform-origin: center center;
}

.mark {
  position: absolute;
  inset: 0;
  margin: auto;
  width: 100%;
  height: 100%;
  padding: 0;
  list-style: none;
}

.mark li {
  position: absolute;
  width: 6px;
  height: 2px;
  background: var(--clock-accent, #ff4d8d);
  transform-origin: 250px;
  box-shadow: 0 0 14px var(--clock-accent, #ff4d8d);
  left: 0;
  top: 0;
}

.mark li.bold {
  width: 8px;
  height: 4px;
}

.numbers {
  position: absolute;
  left: 233px;
  top: 233px;
  font-size: 28px;
  font-weight: 700;
  line-height: 1.5;
  width: 34px;
  height: 34px;
  text-align: center;
  color: var(--clock-accent, #ff4d8d);
  text-shadow: 0 0 14px var(--clock-accent, #ff4d8d);
}

.center {
  position: absolute;
  inset: 0;
  margin: auto;
  width: 24px;
  height: 24px;
  border-radius: 20px;
  background: var(--clock-accent, #ff4d8d);
  box-shadow: 0 0 16px var(--clock-accent, #ff4d8d);
}

.hour-hand,
.minute-hand,
.second-hand {
  background: var(--clock-accent, #ff4d8d);
  box-shadow: 0 0 12px var(--clock-accent, #ff4d8d);
}

.hour-hand {
  position: absolute;
  left: 247px;
  top: 150px;
  width: 6px;
  height: 140px;
  transform-origin: 3px 100px;
}

.minute-hand {
  position: absolute;
  left: 248px;
  top: 70px;
  width: 4px;
  height: 220px;
  transform-origin: 2px 180px;
}

.second-hand {
  position: absolute;
  left: 249px;
  top: 40px;
  width: 2px;
  height: 280px;
  transform-origin: 1px 210px;
}

.time {
  padding: 8px 10px;
  position: absolute;
  left: 50%;
  top: 330px;
  transform: translateX(-50%);
  font-size: 16px;
  font-weight: bold;
  background: transparent;
  color: var(--clock-accent, #ff4d8d);
  text-shadow: 0 0 12px var(--clock-accent, #ff4d8d);
  line-height: 1.4;
  text-align: center;
  white-space: nowrap;
}
</style>

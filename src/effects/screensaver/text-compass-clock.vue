<script setup lang="ts">
/**
 * Text compass clock rewritten from 罗盘时钟 (no jQuery).
 * Ring labels use Arabic digits instead of Chinese numerals.
 */
import { computed, nextTick, onMounted, onUnmounted, shallowRef } from "vue";

type DateInfo = {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  sec: number;
};

const hourItems = Array.from({ length: 24 }, (_, i) => ({
  n: i + 1,
  rotate: (360 / 24) * i * -1,
}));
const minuteItems = Array.from({ length: 60 }, (_, i) => ({
  n: i + 1,
  rotate: (360 / 60) * i * -1,
}));
const secItems = Array.from({ length: 60 }, (_, i) => ({
  n: i + 1,
  rotate: (360 / 60) * i * -1,
}));

const dateInfo = shallowRef<DateInfo>(readNow());
const dateText = shallowRef("");
const hourRotate = shallowRef(0);
const minuteRotate = shallowRef(0);
const secRotate = shallowRef(0);
const hourTransition = shallowRef("transform 0.3s ease-in-out 0s");
const minuteTransition = shallowRef("transform 0.3s ease-in-out 0s");
const secTransition = shallowRef("transform 0.3s ease-in-out 0s");
const hrActive = shallowRef(false);
const ringsReady = shallowRef(false);

let timers: ReturnType<typeof setTimeout>[] = [];
let stopped = false;

function readNow(): DateInfo {
  const now = new Date();
  return {
    year: now.getFullYear(),
    month: now.getMonth() + 1,
    day: now.getDate(),
    hour: now.getHours() || 24,
    minute: now.getMinutes() || 60,
    sec: now.getSeconds() || 60,
  };
}

function schedule(fn: () => void, ms: number) {
  const id = setTimeout(() => {
    if (!stopped) {
      fn();
    }
  }, ms);
  timers.push(id);
}

function formatDate(info: DateInfo) {
  return `${info.year}年${info.month}月${info.day}日`;
}

function syncDateText() {
  dateText.value = formatDate(dateInfo.value);
}

const hourRingStyle = computed(() => ({
  transform: `rotate(${hourRotate.value}deg)`,
  transition: hourTransition.value,
}));
const minuteRingStyle = computed(() => ({
  transform: `rotate(${minuteRotate.value}deg)`,
  transition: minuteTransition.value,
}));
const secRingStyle = computed(() => ({
  transform: `rotate(${secRotate.value}deg)`,
  transition: secTransition.value,
}));

function initRotate() {
  const info = dateInfo.value;
  hourRotate.value = (360 / 24) * (info.hour - 1);
  minuteRotate.value = (360 / 60) * (info.minute - 1);
  secRotate.value = (360 / 60) * (info.sec - 1);
  schedule(() => {
    hrActive.value = true;
    start();
  }, 300);
}

function start() {
  schedule(() => {
    if (stopped) {
      return;
    }
    const info = { ...dateInfo.value };
    if (info.sec <= 60) {
      info.sec += 1;
      dateInfo.value = info;
      secRotate.value = (360 / 60) * (info.sec - 1);
      minuteAdd();
      start();
    }
  }, 1000);
}

function minuteAdd() {
  if (dateInfo.value.sec !== 60 + 1) {
    return;
  }
  schedule(() => {
    secTransition.value = "0s";
    secRotate.value = 0;
    const info = { ...dateInfo.value, sec: 1 };
    dateInfo.value = info;
    schedule(() => {
      secTransition.value = "transform 0.3s ease-in-out 0s";
    }, 100);
    info.minute += 1;
    dateInfo.value = { ...info };
    minuteRotate.value = (360 / 60) * (info.minute - 1);
    hourAdd();
  }, 300);
}

function hourAdd() {
  if (dateInfo.value.minute !== 60 + 1) {
    return;
  }
  schedule(() => {
    minuteTransition.value = "0s";
    minuteRotate.value = 0;
    const info = { ...dateInfo.value, minute: 1 };
    dateInfo.value = info;
    schedule(() => {
      minuteTransition.value = "transform 0.3s ease-in-out 0s";
    }, 100);
    info.hour += 1;
    dateInfo.value = { ...info };
    hourRotate.value = (360 / 24) * (info.hour - 1);
    dayAdd();
  }, 300);
}

function dayAdd() {
  if (dateInfo.value.hour !== 24 + 1) {
    return;
  }
  schedule(() => {
    hourTransition.value = "0s";
    hourRotate.value = 0;
    const now = new Date();
    dateInfo.value = {
      year: now.getFullYear(),
      month: now.getMonth() + 1,
      day: now.getDate(),
      hour: 1,
      minute: dateInfo.value.minute,
      sec: dateInfo.value.sec,
    };
    schedule(() => {
      hourTransition.value = "transform 0.3s ease-in-out 0s";
    }, 100);
    syncDateText();
  }, 300);
}

onMounted(async () => {
  stopped = false;
  dateInfo.value = readNow();
  syncDateText();
  await nextTick();
  // Stagger ring fan-out like the original plugin.
  schedule(() => {
    ringsReady.value = true;
    schedule(() => initRotate(), 1300);
  }, 100);
});

onUnmounted(() => {
  stopped = true;
  for (const id of timers) {
    clearTimeout(id);
  }
  timers = [];
});
</script>

<template>
  <div class="compass-root">
    <ul class="clock">
      <hr :class="{ active: hrActive }" />
      <li class="date">{{ dateText }}</li>
      <li
        class="hour on-hour"
        :class="{ ready: ringsReady }"
        :style="hourRingStyle"
      >
        <div
          v-for="item in hourItems"
          :key="`h-${item.n}`"
          :style="{ transform: ringsReady ? `rotate(${item.rotate}deg)` : undefined }"
        >
          <div>{{ item.n }}时</div>
        </div>
      </li>
      <li
        class="hour minute on-minute"
        :class="{ ready: ringsReady }"
        :style="minuteRingStyle"
      >
        <div
          v-for="item in minuteItems"
          :key="`m-${item.n}`"
          :style="{ transform: ringsReady ? `rotate(${item.rotate}deg)` : undefined }"
        >
          <div>{{ item.n }}分</div>
        </div>
      </li>
      <li
        class="hour sec on-sec"
        :class="{ ready: ringsReady }"
        :style="secRingStyle"
      >
        <div
          v-for="item in secItems"
          :key="`s-${item.n}`"
          :style="{ transform: ringsReady ? `rotate(${item.rotate}deg)` : undefined }"
        >
          <div>{{ item.n }}秒</div>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.compass-root {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  z-index: 5;
  color: #fff;
  font-family: "Microsoft YaHei", "Segoe UI", sans-serif;
  font-size: 14px;
}

.clock {
  list-style: none;
  margin: 0;
  padding: 0;
  width: 700px;
  height: 700px;
  position: relative;
  line-height: 20px;
  user-select: none;
  transform: scale(calc(min(700px, 90vmin) / 700px));
  transform-origin: center center;
}

.date {
  position: absolute;
  z-index: 1;
  width: 100%;
  height: 20px;
  text-align: center;
  top: 340px;
  left: 0;
  text-shadow: 0 0 8px rgba(0, 0, 0, 0.65);
}

.hour {
  position: absolute;
  z-index: 3;
  width: 360px;
  height: 20px;
  top: 340px;
  left: 170px;
  transform: rotate(0deg);
}

.hour > div {
  position: absolute;
  width: 100%;
  right: 0;
  top: 0;
  transition: transform 1s ease-in-out 0s;
  transform: rotate(0deg);
}

.hour > div > div {
  float: right;
  width: 60px;
  text-align: right;
  text-shadow: 0 0 6px rgba(0, 0, 0, 0.55);
}

.minute {
  z-index: 4;
  width: 520px;
  left: 90px;
}

.sec {
  z-index: 5;
  width: 680px;
  left: 10px;
}

.clock > hr {
  height: 0;
  width: 0%;
  position: absolute;
  z-index: 1;
  border: #ffffff solid 0;
  border-bottom-width: 1px;
  margin: 10px 0 0;
  left: 50%;
  top: 50%;
  transition: width 0.3s ease-in-out 0s;
  overflow: visible;
}

.clock > hr.active {
  width: 49%;
}

.clock > hr.active::before {
  content: "";
  display: block;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background-color: yellow;
  top: -2px;
  left: 0;
  position: absolute;
}
</style>

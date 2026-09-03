<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { MeteorTrail } from "@/effects/meteor-trail";
import { RibbonTrail } from "@/effects/ribbon-trail";
import { GraffitiTrail } from "@/effects/graffiti-trail";
import { DotsTrail } from "@/effects/dots-trail";
import { HeartTrail } from "@/effects/heart-trail";
import { RippleTrail } from "@/effects/ripple-trail";
import type { MouseTrailEffect, MouseTrailEngine } from "@/effects/mouse-trail-types";
import { useGlobalTrail } from "../composables/use-global-trail";

const hostRef = ref<HTMLElement | null>(null);
const engine = shallowRef<MouseTrailEngine | null>(null);
const { effect, color } = useGlobalTrail();

function createEngine(next: MouseTrailEffect, trailColor: string): MouseTrailEngine | null {
  const host = hostRef.value;
  if (!host) {
    return null;
  }
  if (next === "meteor") {
    return new MeteorTrail(host, { color: trailColor });
  }
  if (next === "graffiti") {
    return new GraffitiTrail(host);
  }
  if (next === "dots") {
    return new DotsTrail(host, { color: trailColor });
  }
  if (next === "heart") {
    return new HeartTrail(host, { color: trailColor });
  }
  if (next === "ripple") {
    return new RippleTrail(host);
  }
  return new RibbonTrail(host);
}

function rebuild() {
  engine.value?.destroy();
  engine.value = null;
  const next = createEngine(effect.value, color.value);
  if (!next) {
    return;
  }
  engine.value = next;
  next.start();
  next.resize();
}

function onPointerMove(event: PointerEvent) {
  engine.value?.setMouse(event.clientX, event.clientY);
}

function onPointerLeave() {
  engine.value?.leaveScreen();
}

function onResize() {
  engine.value?.resize();
}

watch([effect, color], () => {
  rebuild();
});

onMounted(() => {
  rebuild();
  window.addEventListener("pointermove", onPointerMove, { passive: true });
  window.addEventListener("blur", onPointerLeave);
  document.documentElement.addEventListener("mouseleave", onPointerLeave);
  window.addEventListener("resize", onResize);
});

onUnmounted(() => {
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("blur", onPointerLeave);
  document.documentElement.removeEventListener("mouseleave", onPointerLeave);
  window.removeEventListener("resize", onResize);
  engine.value?.destroy();
  engine.value = null;
});
</script>

<template>
  <div ref="hostRef" class="trail-layer" aria-hidden="true" />
</template>

<style scoped>
.trail-layer {
  position: fixed;
  inset: 0;
  z-index: 40;
  pointer-events: none;
  overflow: hidden;
}
</style>

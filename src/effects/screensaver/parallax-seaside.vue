<script setup lang="ts">
/**
 * Migrated from parallax.js demo (MIT License)
 * Copyright (C) 2014 Matthew Wagerfield - @wagerfield
 */
import { onMounted, onUnmounted, ref } from "vue";
import Parallax from "@/assets/screensaver/parallax/parallax.js";
import ropeUrl from "@/assets/screensaver/parallax/rope.png";
import "@/assets/screensaver/parallax/parallax-seaside.css";
import FluorescentClock from "@/effects/screensaver/fluorescent-clock.vue";

type ParallaxInstance = {
  disable: () => void;
  enable: () => void;
};

const sceneRef = ref<HTMLElement | null>(null);
let parallax: ParallaxInstance | null = null;

function resizeScene() {
  const scene = sceneRef.value;
  if (!scene) {
    return;
  }
  scene.style.width = `${window.innerWidth}px`;
  scene.style.height = `${window.innerHeight}px`;
}

onMounted(() => {
  resizeScene();
  window.addEventListener("resize", resizeScene);
  const scene = sceneRef.value;
  if (!scene) {
    return;
  }
  parallax = new (Parallax as unknown as new (el: HTMLElement) => ParallaxInstance)(scene);
});

onUnmounted(() => {
  window.removeEventListener("resize", resizeScene);
  parallax?.disable();
  parallax = null;
});
</script>

<template>
  <div class="parallax-root">
    <ul
      id="scene"
      ref="sceneRef"
      class="scene unselectable"
      data-friction-x="0.1"
      data-friction-y="0.1"
      data-scalar-x="25"
      data-scalar-y="15"
    >
      <li class="layer" data-depth="0.00" />
      <li class="layer" data-depth="0.10">
        <div class="background" />
      </li>
      <li class="layer" data-depth="0.10">
        <div class="light orange b phase-4" />
      </li>
      <li class="layer" data-depth="0.10">
        <div class="light purple c phase-5" />
      </li>
      <li class="layer" data-depth="0.10">
        <div class="light orange d phase-3" />
      </li>
      <li class="layer" data-depth="0.15">
        <ul class="rope depth-10">
          <li>
            <img :src="ropeUrl" alt="" />
          </li>
          <li class="hanger position-2">
            <div class="board cloud-2 swing-1" />
          </li>
          <li class="hanger position-4">
            <div class="board cloud-1 swing-3" />
          </li>
          <li class="hanger position-8">
            <div class="board birds swing-5" />
          </li>
        </ul>
      </li>
      <li class="layer" data-depth="0.30">
        <ul class="rope depth-30">
          <li>
            <img :src="ropeUrl" alt="" />
          </li>
          <li class="hanger position-1">
            <div class="board cloud-1 swing-3" />
          </li>
          <li class="hanger position-5">
            <div class="board cloud-4 swing-1" />
          </li>
        </ul>
      </li>
      <li class="layer" data-depth="0.30">
        <div class="wave paint depth-30" />
      </li>
      <li class="layer" data-depth="0.40">
        <div class="wave plain depth-40" />
      </li>
      <li class="layer" data-depth="0.50">
        <div class="wave paint depth-50" />
      </li>
      <li class="layer" data-depth="0.60">
        <div class="lighthouse depth-60" />
      </li>
      <li class="layer" data-depth="0.60">
        <ul class="rope depth-60">
          <li>
            <img :src="ropeUrl" alt="" />
          </li>
          <li class="hanger position-3">
            <div class="board birds swing-5" />
          </li>
          <li class="hanger position-6">
            <div class="board cloud-2 swing-2" />
          </li>
          <li class="hanger position-8">
            <div class="board cloud-3 swing-4" />
          </li>
        </ul>
      </li>
      <li class="layer" data-depth="0.60">
        <div class="wave plain depth-60" />
      </li>
      <li class="layer" data-depth="0.80">
        <div class="wave plain depth-80" />
      </li>
      <li class="layer" data-depth="1.00">
        <div class="wave paint depth-100" />
      </li>
    </ul>
    <FluorescentClock />
  </div>
</template>

<style scoped>
.parallax-root {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: #000;
}

.unselectable {
  user-select: none;
}

.rope img {
  display: block;
  width: 100%;
  height: auto;
  pointer-events: none;
}
</style>

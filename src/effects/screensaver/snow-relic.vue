<script setup lang="ts">
/**
 * Mouse-follow snow particles migrated from webPageOfLQY newyear snow3D.js
 * (legacy Three.js CanvasRenderer + Particle3D).
 */
import { onMounted, onUnmounted, ref } from "vue";
import threeUrl from "@/assets/screensaver/snow/three.js?url";
import snow0 from "@/assets/screensaver/snow/151375665240370100.png";
import snow1 from "@/assets/screensaver/snow/151375668550091372.png";
import snow2 from "@/assets/screensaver/snow/151375669416355455.png";
import snow3 from "@/assets/screensaver/snow/151375670204115466.png";
import snow4 from "@/assets/screensaver/snow/151375671039447316.png";

const SNOW_COUNT = 300;
const snowSrcs = [snow0, snow1, snow2, snow3, snow4];

type LegacyThree = {
  PerspectiveCamera: new (
    fov: number,
    aspect: number,
    near: number,
    far: number,
  ) => LegacyCamera;
  Scene: new () => LegacyScene;
  CanvasRenderer: new () => LegacyRenderer;
  ParticleBasicMaterial: new (params: { map: unknown }) => unknown;
  Texture: new (image: HTMLImageElement) => { image: HTMLImageElement; needsUpdate?: boolean };
};

type LegacyCamera = {
  position: { x: number; y: number; z: number };
  lookAt: (target: { x?: number; y?: number; z?: number }) => void;
};

type LegacyScene = {
  add: (obj: unknown) => void;
  position: { x: number; y: number; z: number };
};

type LegacyRenderer = {
  setSize: (w: number, h: number) => void;
  setClearColorHex?: (hex: number, alpha: number) => void;
  setClearColor?: (color: number, alpha: number) => void;
  domElement: HTMLCanvasElement;
  render: (scene: LegacyScene, camera: LegacyCamera) => void;
};

type Particle3DCtor = new (material: unknown) => {
  position: { x: number; y: number; z: number };
  scale: { x: number; y: number };
  updatePhysics: () => void;
};

const containerRef = ref<HTMLElement | null>(null);

let timer: ReturnType<typeof setInterval> | null = null;
let mouseX = 0;
let mouseY = 0;
let windowHalfX = window.innerWidth / 2;
let windowHalfY = window.innerHeight / 2;
let camera: LegacyCamera | null = null;
let scene: LegacyScene | null = null;
let renderer: LegacyRenderer | null = null;
let particles: Array<{
  position: { x: number; y: number; z: number };
  scale: { x: number; y: number };
  updatePhysics: () => void;
}> = [];
let onResize: (() => void) | null = null;
let onMouseMove: ((event: MouseEvent) => void) | null = null;
let scriptEl: HTMLScriptElement | null = null;

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error(`Failed to load ${src}`));
    img.src = src;
  });
}

function ensureThree(url: string): Promise<void> {
  const w = window as Window & { THREE?: LegacyThree; Particle3D?: Particle3DCtor };
  if (w.THREE && w.Particle3D) {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    const existing = document.querySelector<HTMLScriptElement>(
      `script[data-jdd-snow-three="1"]`,
    );
    if (existing) {
      existing.addEventListener("load", () => resolve());
      existing.addEventListener("error", () => reject(new Error("three.js load failed")));
      if (w.THREE && w.Particle3D) {
        resolve();
      }
      return;
    }
    const script = document.createElement("script");
    script.src = url;
    script.async = true;
    script.dataset.jddSnowThree = "1";
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("three.js load failed"));
    document.head.appendChild(script);
    scriptEl = script;
  });
}

function loop() {
  if (!camera || !scene || !renderer) {
    return;
  }
  for (let i = 0; i < particles.length; i++) {
    const particle = particles[i];
    particle.scale.x = particle.scale.y = 1;
    particle.updatePhysics();
    const pos = particle.position;
    if (pos.y < -1000) {
      pos.y += 2000;
    }
    if (pos.x > 1000) {
      pos.x -= 2000;
    } else if (pos.x < -1000) {
      pos.x += 2000;
    }
    if (pos.z > 1000) {
      pos.z -= 2000;
    } else if (pos.z < -1000) {
      pos.z += 2000;
    }
  }
  camera.position.x += (mouseX - camera.position.x) * 0.1;
  camera.position.y += (-mouseY - camera.position.y) * 0.1;
  camera.lookAt(scene.position);
  renderer.render(scene, camera);
}

async function init() {
  const container = containerRef.value;
  if (!container) {
    return;
  }
  await ensureThree(threeUrl);
  const w = window as Window & { THREE?: LegacyThree; Particle3D?: Particle3DCtor };
  const THREE = w.THREE;
  const Particle3D = w.Particle3D;
  if (!THREE || !Particle3D) {
    throw new Error("Legacy THREE / Particle3D unavailable");
  }

  const images = await Promise.all(snowSrcs.map(loadImage));
  const width = container.clientWidth || window.innerWidth;
  const height = container.clientHeight || window.innerHeight;
  windowHalfX = window.innerWidth / 2;
  windowHalfY = window.innerHeight / 2;

  camera = new THREE.PerspectiveCamera(75, width / height, 1, 10000);
  camera.position.z = 1000;
  scene = new THREE.Scene();
  scene.add(camera);
  renderer = new THREE.CanvasRenderer();
  renderer.setClearColorHex?.(0x000000, 0);
  renderer.setSize(width, height);
  renderer.domElement.style.width = "100%";
  renderer.domElement.style.height = "100%";
  renderer.domElement.style.display = "block";
  renderer.domElement.style.background = "transparent";
  container.appendChild(renderer.domElement);

  particles = [];
  for (let i = 0; i < SNOW_COUNT; i++) {
    const texture = new THREE.Texture(images[i % 5]);
    texture.needsUpdate = true;
    const material = new THREE.ParticleBasicMaterial({ map: texture });
    const particle = new Particle3D(material);
    particle.position.x = Math.random() * 2000 - 1000;
    particle.position.y = Math.random() * 2000 - 1000;
    particle.position.z = Math.random() * 2000 - 1000;
    particle.scale.x = particle.scale.y = 1;
    scene.add(particle);
    particles.push(particle);
  }

  onMouseMove = (event: MouseEvent) => {
    mouseX = event.clientX - windowHalfX;
    mouseY = event.clientY - windowHalfY;
  };
  onResize = () => {
    if (!camera || !renderer || !container) {
      return;
    }
    const wSize = container.clientWidth || window.innerWidth;
    const hSize = container.clientHeight || window.innerHeight;
    windowHalfX = window.innerWidth / 2;
    windowHalfY = window.innerHeight / 2;
    (camera as LegacyCamera & { aspect?: number }).aspect = wSize / hSize;
    const cam = camera as LegacyCamera & { updateProjectionMatrix?: () => void };
    cam.updateProjectionMatrix?.();
    renderer.setSize(wSize, hSize);
  };

  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("resize", onResize);
  timer = setInterval(loop, 1000 / 50);
}

function teardown() {
  if (timer !== null) {
    clearInterval(timer);
    timer = null;
  }
  if (onMouseMove) {
    window.removeEventListener("mousemove", onMouseMove);
    onMouseMove = null;
  }
  if (onResize) {
    window.removeEventListener("resize", onResize);
    onResize = null;
  }
  if (renderer?.domElement.parentElement) {
    renderer.domElement.parentElement.removeChild(renderer.domElement);
  }
  particles = [];
  camera = null;
  scene = null;
  renderer = null;
}

onMounted(() => {
  void init().catch((error) => {
    console.error(error);
  });
});

onUnmounted(() => {
  teardown();
  // Keep shared three.js script for reuse across reopen; do not remove scriptEl.
  void scriptEl;
});
</script>

<template>
  <div class="snow-relic-root">
    <div class="snow-bg" aria-hidden="true" />
    <div ref="containerRef" class="snow-layer" />
  </div>
</template>

<style scoped>
.snow-relic-root {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  pointer-events: none;
  background: #070b14;
}

.snow-bg {
  position: absolute;
  inset: 0;
  background: url("@/assets/screensaver/snow/background.jpg") center / cover no-repeat;
}

.snow-layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}
</style>

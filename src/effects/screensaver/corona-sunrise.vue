<script setup lang="ts">
/**
 * Migrated from fg_sunUpDown.html (日出日落 / godrays canvas animation).
 */
import { onMounted, onUnmounted, ref } from "vue";

const canvasRef = ref<HTMLCanvasElement | null>(null);

let rafId = 0;
let frame = 0;
let running = false;
let godraysCanvas: HTMLCanvasElement | null = null;

function mountainHeight(position: number, roughness: number) {
  const frequencies = [1721, 947, 547, 233, 73, 31, 7];
  return frequencies.reduce(
    (height, freq) => height * roughness - Math.cos(freq * position),
    0,
  );
}

function onFrame() {
  if (!running) {
    return;
  }
  const canvas = canvasRef.value;
  if (!canvas || !godraysCanvas) {
    return;
  }
  const ctx = canvas.getContext("2d");
  const godraysCtx = godraysCanvas.getContext("2d");
  if (!ctx || !godraysCtx) {
    return;
  }

  canvas.width = 512;
  canvas.height = 256;
  godraysCanvas.width = 128;
  godraysCanvas.height = 64;

  const sunY = Math.cos(frame++ / 512) * 24;
  const emissionGradient = godraysCtx.createRadialGradient(
    godraysCanvas.width / 2,
    godraysCanvas.height / 2 + sunY,
    0,
    godraysCanvas.width / 2,
    godraysCanvas.height / 2 + sunY,
    44,
  );
  godraysCtx.fillStyle = emissionGradient;
  emissionGradient.addColorStop(0.1, "#0C0804");
  emissionGradient.addColorStop(0.2, "#060201");
  godraysCtx.fillRect(0, 0, godraysCanvas.width, godraysCanvas.height);
  godraysCtx.fillStyle = "#000";

  const skyGradient = ctx.createLinearGradient(0, 0, 0, canvas.height);
  skyGradient.addColorStop(0, "#2a3e55");
  skyGradient.addColorStop(0.7, "#8d4835");
  ctx.fillStyle = skyGradient;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  for (let i = 0; i < 4; i++) {
    ctx.fillStyle = `hsl(7, 23%, ${23 - i * 6}%)`;
    for (let x = canvas.width; x--; ) {
      const mountainPosition = (frame + frame * i * i) / 1000 + x / 2000;
      const mountainRoughness = i / 19 - 0.5;
      const y = 128 + i * 25 + mountainHeight(mountainPosition, mountainRoughness) * 45;
      ctx.fillRect(x, y, 1, 999);
      godraysCtx.fillRect(x / 4, y / 4 + 1, 1, 999);
    }
  }

  ctx.globalCompositeOperation = "lighter";
  godraysCtx.globalCompositeOperation = "lighter";
  for (let scaleFactor = 1.07; scaleFactor < 5; scaleFactor *= scaleFactor) {
    godraysCtx.drawImage(
      godraysCanvas,
      (godraysCanvas.width - godraysCanvas.width * scaleFactor) / 2,
      (godraysCanvas.height - godraysCanvas.height * scaleFactor) / 2 -
        sunY * scaleFactor +
        sunY,
      godraysCanvas.width * scaleFactor,
      godraysCanvas.height * scaleFactor,
    );
  }
  ctx.drawImage(godraysCanvas, 0, 0, canvas.width, canvas.height);
  ctx.globalCompositeOperation = "source-over";

  rafId = window.requestAnimationFrame(onFrame);
}

onMounted(() => {
  const canvas = canvasRef.value;
  if (!canvas) {
    return;
  }
  godraysCanvas = canvas.cloneNode() as HTMLCanvasElement;
  frame = 0;
  running = true;
  rafId = window.requestAnimationFrame(onFrame);
});

onUnmounted(() => {
  running = false;
  window.cancelAnimationFrame(rafId);
  godraysCanvas = null;
});
</script>

<template>
  <div class="corona-root">
    <canvas ref="canvasRef" class="corona-canvas" />
  </div>
</template>

<style scoped>
.corona-root {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: #000;
}

.corona-canvas {
  width: 100%;
  height: 100%;
  object-fit: cover;
  background: #000;
  display: block;
}
</style>

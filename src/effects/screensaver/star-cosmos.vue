<script setup lang="ts">
/**
 * Cosmic starlight: background photo + WebGL star shimmer
 * (ported from HTML5 星光闪烁 3D 视差特效 shaders, without Three.js).
 */
import { onMounted, onUnmounted, ref } from "vue";

const canvasRef = ref<HTMLCanvasElement | null>(null);

/** Cap internal render size to keep fragment cost bounded on 4K / multi-monitor. */
const MAX_INTERNAL_EDGE = 1280;

const VERT = `#version 300 es
in vec2 a_pos;
void main() {
  gl_Position = vec4(a_pos, 0.0, 1.0);
}
`;

const FRAG = `#version 300 es
precision mediump float;

uniform vec2 u_mouse;
uniform vec2 u_resolution;
uniform float u_time;

out vec4 fragColor;

#define PI 3.1415926535897

mat2 r2(float a) {
  return mat2(cos(a), -sin(a), sin(a), cos(a));
}

float hash2(vec2 p) {
  p = fract(p * vec2(125.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float star(vec2 p, float flare) {
  float d = length(p);
  float m = 0.01 / d;
  float col = m;
  float rays = max(0.0, 0.45 - abs(p.x * p.y * 1000.0));
  col += rays * flare;
  p *= r2(PI / 4.0);
  rays = max(0.0, 0.5 - abs(p.x * p.y * 1000.0));
  col += rays * 0.3 * flare;
  col *= smoothstep(0.5, 0.2, d);
  return col;
}

vec3 star_layer(vec2 p) {
  vec3 col = vec3(0.0);
  vec2 pg = fract(p) - 0.5;
  vec2 id = floor(p);
  for (int y = -1; y <= 1; y++) {
    for (int x = -1; x <= 1; x++) {
      vec2 offset = vec2(float(x), float(y));
      float pid = hash2(id + offset);
      float size = fract(pid * 345.32);
      float rn = fract(pid * 645.32);
      vec2 pos = pg - offset - (vec2(pid, fract(pid * 2.0)) - 0.5);
      float s = star(pos, smoothstep(0.86, 1.0, size));
      vec3 wv = vec3(1.0, rn, rn) * sin(pid * 943.321 + u_time * 0.5) * PI;
      vec3 color = sin(wv) * 1.5 + 1.5;
      s *= sin(u_time * 2.5 + pid * 6.2831) * 1.0 + 1.5;
      col += s * size * color;
    }
  }
  return col;
}

void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution.xy) / u_resolution.y;
  vec2 ms = (u_mouse.xy - u_resolution.xy * 0.5) / u_resolution.y;
  uv *= 0.5;
  vec3 col = vec3(0.0);
  uv *= r2(ms.x * 0.25);

  // 3 depth layers (was 5) for lower fill cost.
  for (float i = 0.0; i < 1.0; i += 1.0 / 3.0) {
    float depth = fract(i + u_time * 0.1);
    float scale = mix(20.0, 1.0, depth);
    float fade = depth * smoothstep(1.0, 0.9, depth);
    col += star_layer(uv * scale + (i * 32.0) + ms) * fade;
  }

  fragColor = vec4(col, 1.0);
}
`;

let gl: WebGL2RenderingContext | null = null;
let program: WebGLProgram | null = null;
let rafId = 0;
let running = false;
let startMs = 0;
let mouseX = 0;
let mouseY = 0;
let renderScale = 1;
let locTime: WebGLUniformLocation | null = null;
let locMouse: WebGLUniformLocation | null = null;
let locResolution: WebGLUniformLocation | null = null;

function compile(ctx: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = ctx.createShader(type);
  if (!shader) {
    throw new Error("createShader failed");
  }
  ctx.shaderSource(shader, source);
  ctx.compileShader(shader);
  if (!ctx.getShaderParameter(shader, ctx.COMPILE_STATUS)) {
    const info = ctx.getShaderInfoLog(shader) ?? "shader compile failed";
    ctx.deleteShader(shader);
    throw new Error(info);
  }
  return shader;
}

function linkProgram(ctx: WebGL2RenderingContext): WebGLProgram {
  const vs = compile(ctx, ctx.VERTEX_SHADER, VERT);
  const fs = compile(ctx, ctx.FRAGMENT_SHADER, FRAG);
  const prog = ctx.createProgram();
  if (!prog) {
    throw new Error("createProgram failed");
  }
  ctx.attachShader(prog, vs);
  ctx.attachShader(prog, fs);
  ctx.linkProgram(prog);
  ctx.deleteShader(vs);
  ctx.deleteShader(fs);
  if (!ctx.getProgramParameter(prog, ctx.LINK_STATUS)) {
    const info = ctx.getProgramInfoLog(prog) ?? "program link failed";
    ctx.deleteProgram(prog);
    throw new Error(info);
  }
  return prog;
}

function resize() {
  const canvas = canvasRef.value;
  if (!canvas || !gl || !program) {
    return;
  }
  const cssW = Math.max(1, canvas.clientWidth);
  const cssH = Math.max(1, canvas.clientHeight);
  const longEdge = Math.max(cssW, cssH);
  renderScale = longEdge > MAX_INTERNAL_EDGE ? MAX_INTERNAL_EDGE / longEdge : 1;
  const w = Math.max(1, Math.floor(cssW * renderScale));
  const h = Math.max(1, Math.floor(cssH * renderScale));
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
  }
  gl.useProgram(program);
  gl.viewport(0, 0, w, h);
  gl.uniform2f(locResolution, w, h);
}

function onMouseMove(event: MouseEvent) {
  const canvas = canvasRef.value;
  if (!canvas) {
    return;
  }
  const rect = canvas.getBoundingClientRect();
  // Match reference demo coords, scaled to internal buffer.
  mouseX = (event.clientX - rect.left) * renderScale;
  mouseY = (event.clientY - rect.top) * renderScale;
}

function frame(now: number) {
  if (!running || !gl || !program) {
    return;
  }
  if (!startMs) {
    startMs = now;
  }
  gl.useProgram(program);
  gl.uniform1f(locTime, (now - startMs) / 1000);
  gl.uniform2f(locMouse, mouseX, mouseY);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
  rafId = requestAnimationFrame(frame);
}

function init() {
  const canvas = canvasRef.value;
  if (!canvas) {
    return;
  }
  const ctx = canvas.getContext("webgl2", {
    alpha: false,
    antialias: false,
    depth: false,
    stencil: false,
    powerPreference: "low-power",
  });
  if (!ctx) {
    return;
  }
  gl = ctx;
  program = linkProgram(ctx);
  ctx.useProgram(program);

  const buf = ctx.createBuffer();
  ctx.bindBuffer(ctx.ARRAY_BUFFER, buf);
  // Fullscreen triangle covering NDC
  ctx.bufferData(
    ctx.ARRAY_BUFFER,
    new Float32Array([-1, -1, 3, -1, -1, 3]),
    ctx.STATIC_DRAW,
  );
  const locPos = ctx.getAttribLocation(program, "a_pos");
  ctx.enableVertexAttribArray(locPos);
  ctx.vertexAttribPointer(locPos, 2, ctx.FLOAT, false, 0, 0);

  locTime = ctx.getUniformLocation(program, "u_time");
  locMouse = ctx.getUniformLocation(program, "u_mouse");
  locResolution = ctx.getUniformLocation(program, "u_resolution");

  resize();
  mouseX = canvas.width / 2;
  mouseY = canvas.height / 2;

  running = true;
  startMs = 0;
  rafId = requestAnimationFrame(frame);
}

function dispose() {
  running = false;
  if (rafId) {
    cancelAnimationFrame(rafId);
    rafId = 0;
  }
  if (gl) {
    if (program) {
      gl.deleteProgram(program);
    }
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  }
  program = null;
  gl = null;
}

onMounted(() => {
  try {
    init();
  } catch {
    dispose();
  }
  window.addEventListener("resize", resize);
  window.addEventListener("mousemove", onMouseMove);
});

onUnmounted(() => {
  window.removeEventListener("resize", resize);
  window.removeEventListener("mousemove", onMouseMove);
  dispose();
});
</script>

<template>
  <div class="star-cosmos-root">
    <div class="star-bg" aria-hidden="true" />
    <canvas ref="canvasRef" class="star-canvas" />
  </div>
</template>

<style scoped>
.star-cosmos-root {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  pointer-events: none;
  background: #05070f;
}

.star-bg {
  position: absolute;
  inset: 0;
  background: url("@/assets/screensaver/star/xingkongbg1.jpg") center / cover no-repeat;
}

.star-canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
  mix-blend-mode: screen;
}
</style>

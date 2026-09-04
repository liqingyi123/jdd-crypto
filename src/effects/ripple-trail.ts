/**
 * Procedural water ripple trail (沧涟曳逝).
 *
 * Height-field + refraction adapted from Neil Wallis / Sergey Chikuyonok /
 * Niklas Knaack WaterRippleEffect (MIT). Calm pixels stay fully transparent —
 * only mouse-driven ripples are drawn (no full-screen veil).
 *
 * Ripple color uses soft mid-cool gray with mild highlight/shadow so rings stay
 * readable on dark desktops without staining white ones. Not user-customizable.
 */
import type { MouseTrailEngine } from "./mouse-trail-types";

/** Mid caustic base (refracted for texture). */
const BASE_RGB: [number, number, number] = [168, 172, 178];
/** Crest — soft light gray (readable on dark UI, gentle on light). */
const HIGHLIGHT_RGB: [number, number, number] = [208, 212, 218];
/** Trough — mid cool gray (avoid near-black stains on white desktops). */
const SHADOW_RGB: [number, number, number] = [128, 132, 138];

const SIM_SCALE = 0.45;
const MAX_SIM_WIDTH = 960;
/** Soft circular kernel radius (sim pixels). */
const RIPPLE_RADIUS = 6;
/** Path sample spacing — denser than radius so stamps fuse into a continuous band. */
const DISTURB_STEP = 2.5;
/** Peak impulse at kernel center; soft falloff keeps dense samples from overdriving. */
const DISTURB_IMPULSE = 280;
/** Below this |height|, pixel stays invisible. */
const ENERGY_FLOOR = 8;
const ENERGY_SCALE = 200;
/** Soft overall opacity — lower avoids harsh ink-like stains on white. */
const MAX_RIPPLE_ALPHA = 0.22;
/** Troughs draw lighter than crests so white backgrounds stay clean. */
const TROUGH_ALPHA_SCALE = 0.62;
/** Lower shift → stronger damping (fewer oscillation cycles). */
const DECAY_SHIFT = 4;

function hashNoise(x: number, y: number): number {
  const n = Math.sin(x * 127.1 + y * 311.7) * 43758.5453;
  return n - Math.floor(n);
}

function smoothNoise(x: number, y: number): number {
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  const fx = x - x0;
  const fy = y - y0;
  const u = fx * fx * (3 - 2 * fx);
  const v = fy * fy * (3 - 2 * fy);
  const a = hashNoise(x0, y0);
  const b = hashNoise(x0 + 1, y0);
  const c = hashNoise(x0, y0 + 1);
  const d = hashNoise(x0 + 1, y0 + 1);
  return a * (1 - u) * (1 - v) + b * u * (1 - v) + c * (1 - u) * v + d * u * v;
}

function fbm(x: number, y: number): number {
  let amp = 0.5;
  let freq = 1;
  let sum = 0;
  for (let i = 0; i < 4; i += 1) {
    sum += amp * smoothNoise(x * freq, y * freq);
    amp *= 0.5;
    freq *= 2.05;
  }
  return sum;
}

export class RippleTrail implements MouseTrailEngine {
  private readonly host: HTMLElement;
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;

  private simW = 0;
  private simH = 0;
  private scaleX = 1;
  private scaleY = 1;

  /** Internal caustic luminance map for refraction (never shown as a veil). */
  private source: Uint8ClampedArray = new Uint8ClampedArray(0);
  private output: ImageData | null = null;
  private wave: Int16Array = new Int16Array(0);
  private lastMap: Int16Array = new Int16Array(0);
  private oldPage = 0;
  private newPage = 0;

  private rafId = 0;
  private running = false;
  private lastMx = -1;
  private lastMy = -1;

  constructor(host: HTMLElement) {
    this.host = host;

    this.canvas = document.createElement("canvas");
    this.canvas.style.cssText =
      "position:absolute;inset:0;width:100%;height:100%;pointer-events:none;";
    host.appendChild(this.canvas);
    const ctx = this.canvas.getContext("2d", { alpha: true });
    if (!ctx) {
      throw new Error("Canvas 2D unavailable");
    }
    this.ctx = ctx;
    this.resize();
  }

  setMouse(x: number, y: number): void {
    const sx = x * this.scaleX;
    const sy = y * this.scaleY;
    if (this.lastMx < 0) {
      this.lastMx = sx;
      this.lastMy = sy;
      this.disturb(sx, sy);
      return;
    }

    const dx = sx - this.lastMx;
    const dy = sy - this.lastMy;
    const dist = Math.sqrt(dx * dx + dy * dy);
    if (dist < 0.5) {
      return;
    }

    const steps = Math.max(1, Math.ceil(dist / DISTURB_STEP));
    const inv = 1 / steps;
    for (let s = 1; s <= steps; s += 1) {
      const t = s * inv;
      this.disturb(this.lastMx + dx * t, this.lastMy + dy * t);
    }
    this.lastMx = sx;
    this.lastMy = sy;
  }

  leaveScreen(): void {
    this.lastMx = -1;
    this.lastMy = -1;
  }

  start(): void {
    if (this.running) {
      return;
    }
    this.running = true;
    const loop = () => {
      if (!this.running) {
        return;
      }
      this.step();
      this.rafId = requestAnimationFrame(loop);
    };
    this.rafId = requestAnimationFrame(loop);
  }

  stop(): void {
    this.running = false;
    if (this.rafId) {
      cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
  }

  destroy(): void {
    this.stop();
    this.canvas.remove();
  }

  resize(): void {
    const rect = this.host.getBoundingClientRect();
    const cssW = Math.max(1, Math.floor(rect.width));
    const cssH = Math.max(1, Math.floor(rect.height));

    let simW = Math.max(32, Math.floor(cssW * SIM_SCALE));
    let simH = Math.max(32, Math.floor(cssH * SIM_SCALE));
    if (simW > MAX_SIM_WIDTH) {
      const ratio = MAX_SIM_WIDTH / simW;
      simW = MAX_SIM_WIDTH;
      simH = Math.max(32, Math.floor(simH * ratio));
    }
    this.simW = simW;
    this.simH = simH;
    this.scaleX = simW / cssW;
    this.scaleY = simH / cssH;

    this.canvas.width = simW;
    this.canvas.height = simH;

    const size = simW * (simH + 2) * 2;
    this.wave = new Int16Array(size);
    this.lastMap = new Int16Array(simW * simH);
    this.oldPage = simW;
    this.newPage = simW * (simH + 3);
    this.source = new Uint8ClampedArray(simW * simH * 4);
    this.output = this.ctx.createImageData(simW, simH);
    this.lastMx = -1;
    this.lastMy = -1;
    this.rebuildSource();
  }

  private rebuildSource(): void {
    const w = this.simW;
    const h = this.simH;
    if (w <= 0 || h <= 0) {
      return;
    }
    const [r0, g0, b0] = BASE_RGB;
    const data = this.source;

    for (let y = 0; y < h; y += 1) {
      for (let x = 0; x < w; x += 1) {
        const n = fbm(x / 48, y / 48);
        const caustic = Math.sin((x + y) * 0.085 + n * 6) * 0.5 + 0.5;
        const streak = Math.sin(x * 0.12 - y * 0.07) * 0.5 + 0.5;
        const shade = 0.72 + 0.2 * n + 0.12 * caustic + 0.08 * streak;
        const i = (y * w + x) * 4;
        data[i] = Math.min(255, Math.round(r0 * shade));
        data[i + 1] = Math.min(255, Math.round(g0 * shade));
        data[i + 2] = Math.min(255, Math.round(b0 * shade));
        data[i + 3] = 255;
      }
    }
  }

  private disturb(x: number, y: number): void {
    const w = this.simW;
    const h = this.simH;
    if (w <= 0 || h <= 0) {
      return;
    }
    const r = RIPPLE_RADIUS;
    const rSq = r * r;
    const base = this.oldPage;
    const wave = this.wave;
    const x0 = Math.max(0, Math.floor(x - r));
    const x1 = Math.min(w - 1, Math.ceil(x + r));
    const y0 = Math.max(0, Math.floor(y - r));
    const y1 = Math.min(h - 1, Math.ceil(y + r));
    for (let j = y0; j <= y1; j += 1) {
      const dy = j + 0.5 - y;
      for (let i = x0; i <= x1; i += 1) {
        const dx = i + 0.5 - x;
        const dSq = dx * dx + dy * dy;
        if (dSq >= rSq) {
          continue;
        }
        const falloff = 1 - dSq / rSq;
        wave[base + j * w + i] += (DISTURB_IMPULSE * falloff * falloff) | 0;
      }
    }
  }

  private step(): void {
    const w = this.simW;
    const h = this.simH;
    const out = this.output;
    if (!out || w <= 0 || h <= 0) {
      return;
    }

    let oldPage = this.oldPage;
    let newPage = this.newPage;
    const swap = oldPage;
    oldPage = newPage;
    newPage = swap;
    this.oldPage = oldPage;
    this.newPage = newPage;

    const wave = this.wave;
    const lastMap = this.lastMap;
    const src = this.source;
    const dst = out.data;
    const halfW = w / 2;
    const halfH = h / 2;
    let pixel = 0;
    let cursor = oldPage;

    for (let y = 0; y < h; y += 1) {
      for (let x = 0; x < w; x += 1) {
        let height =
          (wave[cursor - w] + wave[cursor + w] + wave[cursor - 1] + wave[cursor + 1]) >> 1;
        height -= wave[newPage + pixel];
        height -= height >> DECAY_SHIFT;
        wave[newPage + pixel] = height;

        const refract = 1024 - height;
        lastMap[pixel] = refract;

        const dstI = pixel * 4;
        const absH = Math.abs(height);
        if (absH < ENERGY_FLOOR) {
          dst[dstI] = 0;
          dst[dstI + 1] = 0;
          dst[dstI + 2] = 0;
          dst[dstI + 3] = 0;
          cursor += 1;
          pixel += 1;
          continue;
        }

        let sampleX = ((((x - halfW) * refract) / 1024) | 0) + halfW;
        let sampleY = ((((y - halfH) * refract) / 1024) | 0) + halfH;
        if (sampleX >= w) {
          sampleX = w - 1;
        }
        if (sampleX < 0) {
          sampleX = 0;
        }
        if (sampleY >= h) {
          sampleY = h - 1;
        }
        if (sampleY < 0) {
          sampleY = 0;
        }

        const srcI = (sampleX + sampleY * w) * 4;
        const energy = Math.min(1, absH / ENERGY_SCALE);
        // Quadratic falloff softens outer rings; troughs use lighter alpha on white UIs.
        const soft = energy * energy;
        const phase = height > 0 ? 1 : -1;
        const tone = (phase + 1) * 0.5;
        const baseMix = 0.62;
        const r =
          src[srcI] * baseMix +
          HIGHLIGHT_RGB[0] * tone * (1 - baseMix) +
          SHADOW_RGB[0] * (1 - tone) * (1 - baseMix);
        const g =
          src[srcI + 1] * baseMix +
          HIGHLIGHT_RGB[1] * tone * (1 - baseMix) +
          SHADOW_RGB[1] * (1 - tone) * (1 - baseMix);
        const b =
          src[srcI + 2] * baseMix +
          HIGHLIGHT_RGB[2] * tone * (1 - baseMix) +
          SHADOW_RGB[2] * (1 - tone) * (1 - baseMix);

        const alphaScale = height < 0 ? TROUGH_ALPHA_SCALE : 1;
        dst[dstI] = Math.min(255, Math.round(r));
        dst[dstI + 1] = Math.min(255, Math.round(g));
        dst[dstI + 2] = Math.min(255, Math.round(b));
        dst[dstI + 3] = Math.min(
          255,
          Math.round(soft * MAX_RIPPLE_ALPHA * alphaScale * 255),
        );

        cursor += 1;
        pixel += 1;
      }
    }

    this.ctx.clearRect(0, 0, w, h);
    this.ctx.putImageData(out, 0, 0);
  }
}

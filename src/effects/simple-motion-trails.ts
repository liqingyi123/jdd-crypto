/**
 * Simple cursor trails from Canvas跟随鼠标光标动画特效 effects 1 & 2.
 * Transparent overlay (clearRect only — no opaque background fill).
 */
import type { MouseTrailEngine } from "./mouse-trail-types";
import { createFrameGate } from "./trail-frame-budget";

const MAX_PARTICLES = 100;
const R_GROW = 0.9;
const O_DECAY = 0.05;
const FALL_RADIUS = 10;
const FALL_ACCEL = 1;
const FALL_SHADOW = 20;

interface Particle {
  x: number;
  y: number;
  r: number;
  o: number;
  v: number;
}

abstract class SimpleMotionTrail implements MouseTrailEngine {
  protected readonly canvas: HTMLCanvasElement;
  protected readonly ctx: CanvasRenderingContext2D;
  protected color: string;
  protected readonly particles: Particle[] = [];
  protected rafId = 0;
  protected running = false;
  protected readonly frameGate = createFrameGate();

  constructor(host: HTMLElement, color: string) {
    this.color = color;
    this.canvas = document.createElement("canvas");
    this.canvas.style.cssText =
      "position:absolute;inset:0;width:100%;height:100%;pointer-events:none;";
    host.appendChild(this.canvas);
    const ctx = this.canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Canvas 2D unavailable");
    }
    this.ctx = ctx;
    this.resize();
    window.addEventListener("resize", this.onResize);
  }

  private onResize = () => {
    this.resize();
  };

  resize() {
    const width = this.canvas.clientWidth || window.innerWidth;
    const height = this.canvas.clientHeight || window.innerHeight;
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
  }

  setColor(color: string) {
    this.color = color;
  }

  setMouse(x: number, y: number) {
    this.particles.push({ x, y, r: R_GROW, o: 1, v: 0 });
    while (this.particles.length > MAX_PARTICLES) {
      this.particles.shift();
    }
    this.kick();
  }

  leaveScreen() {
    // Stop spawning (no more setMouse); let particles finish fading.
    if (this.particles.length > 0) {
      this.kick();
    }
  }

  start() {
    // Idle until first setMouse.
  }

  setTargetFps(fps: number) {
    this.frameGate.setTargetFps(fps);
  }

  protected kick() {
    if (this.running) {
      return;
    }
    this.running = true;
    const tick = (now: number) => {
      if (!this.running) {
        return;
      }
      if (this.frameGate.shouldRun(now)) {
        this.render();
      }
      if (this.particles.length === 0) {
        this.stopLoop(false);
        return;
      }
      this.rafId = window.requestAnimationFrame(tick);
    };
    this.rafId = window.requestAnimationFrame(tick);
  }

  protected stopLoop(clearParticles: boolean) {
    this.running = false;
    if (this.rafId) {
      window.cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
    if (clearParticles) {
      this.particles.length = 0;
    }
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  stop() {
    this.stopLoop(true);
  }

  destroy() {
    this.stop();
    window.removeEventListener("resize", this.onResize);
    this.canvas.remove();
  }

  protected abstract render(): void;
}

/** 简约涟漪 — expanding fading rings (effect 1). */
export class BloomTrail extends SimpleMotionTrail {
  constructor(host: HTMLElement, options: { color?: string } = {}) {
    super(host, options.color ?? "#7AD7FF");
  }

  protected render() {
    const ctx = this.ctx;
    const width = this.canvas.width;
    const height = this.canvas.height;
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = this.color;

    for (let i = this.particles.length - 1; i >= 0; i -= 1) {
      const p = this.particles[i];
      ctx.globalAlpha = Math.max(0, p.o);
      ctx.beginPath();
      ctx.arc(p.x, p.y, Math.max(0.1, p.r), 0, Math.PI * 2);
      ctx.closePath();
      ctx.fill();
      p.r += R_GROW;
      p.o -= O_DECAY;
      if (p.o <= 0) {
        this.particles.splice(i, 1);
      }
    }
    ctx.globalAlpha = 1;
  }
}

/** 简约坠落 — glowing drops that fall with gravity (effect 2). */
export class FallTrail extends SimpleMotionTrail {
  constructor(host: HTMLElement, options: { color?: string } = {}) {
    super(host, options.color ?? "#FFB86B");
  }

  protected render() {
    const ctx = this.ctx;
    const width = this.canvas.width;
    const height = this.canvas.height;
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = this.color;
    ctx.shadowBlur = FALL_SHADOW;
    ctx.shadowColor = this.color;

    for (let i = this.particles.length - 1; i >= 0; i -= 1) {
      const p = this.particles[i];
      ctx.globalAlpha = Math.max(0, p.o);
      ctx.beginPath();
      ctx.arc(p.x, p.y, FALL_RADIUS, 0, Math.PI * 2);
      ctx.closePath();
      ctx.fill();
      p.o -= O_DECAY;
      p.v += FALL_ACCEL;
      p.y += p.v;
      if (p.y >= height + FALL_RADIUS || p.o <= 0) {
        this.particles.splice(i, 1);
      }
    }
    ctx.shadowBlur = 0;
    ctx.globalAlpha = 1;
  }
}

/**
 * Colorful bubbles trail — mouse Emitter only from the Canvas particle demo
 * (no full-screen Field). Random HSL discs that drift outward and shrink.
 */
import type { MouseTrailEngine } from "./mouse-trail-types";
import { createFrameGate } from "./trail-frame-budget";

interface Bubble {
  x: number;
  y: number;
  speed: number;
  angle: number;
  diaSet: number;
  dia: number;
  age: number;
  fill: string;
  stroke: boolean;
}

const DEFAULT_LIFE = 80;
/** Min cursor travel (px) since last spawn before emitting again. */
const MOVE_SPAWN_EPS = 8;
/** Particles emitted per qualifying move. */
const SPAWN_PER_MOVE = 1;

export class BubblesTrail implements MouseTrailEngine {
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly life: number;
  private readonly particles: Bubble[] = [];
  private mouseX: number | null = null;
  private mouseY: number | null = null;
  private lastSpawnX: number | null = null;
  private lastSpawnY: number | null = null;
  private spawnPending = false;
  private rafId = 0;
  private running = false;
  private readonly frameGate = createFrameGate();

  constructor(host: HTMLElement, options: { life?: number } = {}) {
    this.life = Math.max(10, options.life ?? DEFAULT_LIFE);
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

  setMouse(x: number, y: number) {
    this.mouseX = x;
    this.mouseY = y;
    if (this.lastSpawnX === null || this.lastSpawnY === null) {
      this.spawnPending = true;
    } else {
      const dx = x - this.lastSpawnX;
      const dy = y - this.lastSpawnY;
      if (dx * dx + dy * dy >= MOVE_SPAWN_EPS * MOVE_SPAWN_EPS) {
        this.spawnPending = true;
      }
    }
    this.kick();
  }

  leaveScreen() {
    // Stop spawning; let existing bubbles age out (smooth across monitors).
    this.mouseX = null;
    this.mouseY = null;
    this.lastSpawnX = null;
    this.lastSpawnY = null;
    this.spawnPending = false;
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

  private kick() {
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
      // Idle (no pending spawn, no particles): stop RAF until next move.
      if (!this.spawnPending && this.particles.length === 0) {
        this.stop();
        return;
      }
      this.rafId = window.requestAnimationFrame(tick);
    };
    this.rafId = window.requestAnimationFrame(tick);
  }

  stop() {
    this.running = false;
    if (this.rafId) {
      window.cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  destroy() {
    this.stop();
    this.particles.length = 0;
    window.removeEventListener("resize", this.onResize);
    this.canvas.remove();
  }

  private spawn(x: number, y: number): Bubble {
    const hue = Math.floor(Math.random() * 360);
    const diaSet = 2 + Math.random() * 10;
    return {
      x,
      y,
      speed: Math.random() * 0.5 + 0.2,
      angle: Math.random() * 360,
      diaSet,
      dia: diaSet,
      age: 0,
      fill: `hsl(${hue}, 95%, 70%)`,
      stroke: Math.random() > 0.5,
    };
  }

  private render() {
    const ctx = this.ctx;
    const width = this.canvas.width;
    const height = this.canvas.height;
    ctx.clearRect(0, 0, width, height);

    if (this.spawnPending && this.mouseX !== null && this.mouseY !== null) {
      const mx = this.mouseX;
      const my = this.mouseY;
      for (let n = 0; n < SPAWN_PER_MOVE; n += 1) {
        const jx = mx + (Math.random() - 0.5) * 6;
        const jy = my + (Math.random() - 0.5) * 6;
        this.particles.push(this.spawn(jx, jy));
      }
      this.lastSpawnX = mx;
      this.lastSpawnY = my;
      this.spawnPending = false;
      while (this.particles.length > this.life) {
        this.particles.shift();
      }
    }

    const life = this.life;
    for (let i = this.particles.length - 1; i >= 0; i -= 1) {
      const p = this.particles[i];
      const s = p.speed * 2;
      p.angle += Math.random() * 10 - 5;
      p.x += s * Math.cos((p.angle * Math.PI) / 180);
      p.y += s * Math.sin((p.angle * Math.PI) / 180);
      p.age += 1 / life;
      p.dia = (1 - p.age) * p.diaSet;

      if (p.age >= 1 || p.dia <= 0.15 || p.x < -20 || p.y < -20 || p.x > width + 20 || p.y > height + 20) {
        this.particles.splice(i, 1);
        continue;
      }

      ctx.beginPath();
      ctx.fillStyle = p.fill;
      ctx.strokeStyle = p.fill;
      ctx.lineWidth = 2;
      ctx.arc(p.x, p.y, Math.max(0.1, p.dia), 0, Math.PI * 2);
      ctx.closePath();
      if (p.stroke) {
        ctx.stroke();
      } else {
        ctx.fill();
      }
    }
  }
}

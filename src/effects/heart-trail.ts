import type { MouseTrailEngine } from "./mouse-trail-types";

export interface HeartTrailOptions {
  color?: string;
}

interface GlowNode {
  x: number;
  y: number;
  life: number;
}

interface HeartStamp {
  x: number;
  y: number;
  life: number;
  size: number;
  angle: number;
}

const DEFAULT_COLOR = "#FF2EC8";
/** Dense enough for a continuous nebula band. */
const GLOW_GAP = 6;
/** Hearts are sparse accents — path is carried by glow. */
const HEART_GAP = 40;
const MAX_GLOW = 140;
const MAX_HEART = 18;
/** ~1s fade — longer linger for a smoother continuous trail. */
const LIFE_DECAY = 0.018;
const GLOW_SPRITE = 96;
const HEART_SPRITE = 40;

function parseRgb(input: string): [number, number, number] {
  const hex = input.replace(/^\s*#|\s*$/g, "").toLowerCase();
  if (/^([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/.test(hex)) {
    return [
      parseInt(hex.slice(0, 2), 16),
      parseInt(hex.slice(2, 4), 16),
      parseInt(hex.slice(4, 6), 16),
    ];
  }
  return [255, 46, 200];
}

function createSoftGlowSprite(color: string, size: number): HTMLCanvasElement {
  const [r, g, b] = parseRgb(color);
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return canvas;
  }
  const c = size / 2;
  const grad = ctx.createRadialGradient(c, c, 0, c, c, c);
  grad.addColorStop(0, `rgba(${r},${g},${b},0.7)`);
  grad.addColorStop(0.25, `rgba(${r},${g},${b},0.4)`);
  grad.addColorStop(0.55, `rgba(${r},${g},${b},0.14)`);
  grad.addColorStop(1, `rgba(${r},${g},${b},0)`);
  ctx.fillStyle = grad;
  ctx.beginPath();
  ctx.arc(c, c, c, 0, Math.PI * 2);
  ctx.fill();
  return canvas;
}

function createHeartSprite(color: string, size: number): HTMLCanvasElement {
  const [r, g, b] = parseRgb(color);
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return canvas;
  }
  const s = size;
  ctx.fillStyle = `rgb(${r},${g},${b})`;
  ctx.beginPath();
  ctx.moveTo(0.5 * s, 0.32 * s);
  ctx.bezierCurveTo(0.5 * s, 0.2 * s, 0.32 * s, 0.1 * s, 0.2 * s, 0.22 * s);
  ctx.bezierCurveTo(0.04 * s, 0.36 * s, 0.08 * s, 0.6 * s, 0.5 * s, 0.9 * s);
  ctx.bezierCurveTo(0.92 * s, 0.6 * s, 0.96 * s, 0.36 * s, 0.8 * s, 0.22 * s);
  ctx.bezierCurveTo(0.68 * s, 0.1 * s, 0.5 * s, 0.2 * s, 0.5 * s, 0.32 * s);
  ctx.closePath();
  ctx.fill();
  return canvas;
}

/**
 * Dense path hearts + continuous soft glow ribbon (pre-baked sprites, no live blur).
 */
export class HeartTrail implements MouseTrailEngine {
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private glowSprite: HTMLCanvasElement;
  private heartSprite: HTMLCanvasElement;
  private glow: GlowNode[] = [];
  private hearts: HeartStamp[] = [];
  private lastX = 0;
  private lastY = 0;
  private hasLast = false;
  private glowCarry = 0;
  private heartCarry = 0;
  private running = false;
  private rafId = 0;

  constructor(host: HTMLElement, options: HeartTrailOptions = {}) {
    const color = options.color ?? DEFAULT_COLOR;
    this.glowSprite = createSoftGlowSprite(color, GLOW_SPRITE);
    this.heartSprite = createHeartSprite(color, HEART_SPRITE);

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

  setColor(color: string) {
    this.glowSprite = createSoftGlowSprite(color, GLOW_SPRITE);
    this.heartSprite = createHeartSprite(color, HEART_SPRITE);
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
    if (!this.hasLast) {
      this.lastX = x;
      this.lastY = y;
      this.hasLast = true;
      return;
    }

    const dx = x - this.lastX;
    const dy = y - this.lastY;
    const dist = Math.hypot(dx, dy);
    if (dist < 0.5) {
      return;
    }

    const ux = dx / dist;
    const uy = dy / dist;

    // Walk the segment so fast strokes still leave a continuous band.
    let traveled = 0;
    while (traveled + 0.01 < dist) {
      const stepGlow = GLOW_GAP - this.glowCarry;
      const stepHeart = HEART_GAP - this.heartCarry;
      const step = Math.min(stepGlow, stepHeart, dist - traveled);
      traveled += step;
      this.glowCarry += step;
      this.heartCarry += step;
      const px = this.lastX + ux * traveled;
      const py = this.lastY + uy * traveled;

      if (this.glowCarry >= GLOW_GAP) {
        this.glowCarry -= GLOW_GAP;
        this.pushGlow(px, py);
      }
      if (this.heartCarry >= HEART_GAP) {
        this.heartCarry -= HEART_GAP;
        // Force left/right of the glow band — never on the centerline.
        const nx = -uy;
        const ny = ux;
        const side = Math.random() < 0.5 ? -1 : 1;
        const dist = 6 + Math.random() * 18;
        const offset = dist * side;
        // Outer hearts run smaller (dist 6→24 → size larger→smaller).
        const outer = (dist - 6) / 18;
        const size = 24 - outer * 12 + Math.random() * 3;
        this.pushHeart(
          px + nx * offset,
          py + ny * offset,
          Math.random() * Math.PI * 2,
          size,
        );
      }
    }

    this.lastX = x;
    this.lastY = y;
    this.kick();
  }

  leaveScreen() {
    this.hasLast = false;
    this.glowCarry = 0;
    this.heartCarry = 0;
    this.glow = [];
    this.hearts = [];
    this.stopLoop(true);
  }

  start() {
    // Idle until moving.
  }

  stop() {
    this.hasLast = false;
    this.glowCarry = 0;
    this.heartCarry = 0;
    this.glow = [];
    this.hearts = [];
    this.stopLoop(true);
  }

  destroy() {
    this.stop();
    window.removeEventListener("resize", this.onResize);
    this.canvas.remove();
  }

  private pushGlow(x: number, y: number) {
    if (this.glow.length >= MAX_GLOW) {
      this.glow.shift();
    }
    this.glow.push({ x, y, life: 1 });
  }

  private pushHeart(x: number, y: number, angle: number, size: number) {
    if (this.hearts.length >= MAX_HEART) {
      this.hearts.shift();
    }
    this.hearts.push({
      x,
      y,
      life: 1,
      size,
      angle,
    });
  }

  private kick() {
    if (this.running) {
      return;
    }
    this.running = true;
    const tick = () => {
      if (!this.running) {
        return;
      }
      if (!this.render()) {
        this.running = false;
        this.rafId = 0;
        return;
      }
      this.rafId = window.requestAnimationFrame(tick);
    };
    this.rafId = window.requestAnimationFrame(tick);
  }

  private stopLoop(clear: boolean) {
    this.running = false;
    if (this.rafId) {
      window.cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
    if (clear) {
      this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    }
  }

  private age(list: Array<{ life: number }>) {
    for (let i = list.length - 1; i >= 0; i -= 1) {
      list[i].life -= LIFE_DECAY;
      if (list[i].life <= 0) {
        list.splice(i, 1);
      }
    }
  }

  private render(): boolean {
    this.age(this.glow);
    this.age(this.hearts);

    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    if (this.glow.length === 0 && this.hearts.length === 0) {
      return false;
    }

    // Continuous magenta ribbon — head (cursor / last) bright, tail faint.
    ctx.globalCompositeOperation = "lighter";
    const glowCount = this.glow.length;
    for (let i = 0; i < glowCount; i += 1) {
      const node = this.glow[i];
      const along = glowCount <= 1 ? 1 : i / (glowCount - 1);
      // Milder head→tail contrast than along².
      const fade = along;
      const size = (40 + 28 * fade) * (0.55 + 0.45 * node.life);
      const half = size * 0.5;
      ctx.globalAlpha = 0.48 * node.life * (0.35 + 0.65 * fade);
      ctx.drawImage(this.glowSprite, node.x - half, node.y - half, size, size);
    }

    // Hearts on top of the glow band.
    ctx.globalCompositeOperation = "source-over";
    for (const heart of this.hearts) {
      const size = heart.size * (0.55 + 0.45 * heart.life);
      const half = size * 0.5;
      ctx.globalAlpha = heart.life;
      ctx.setTransform(1, 0, 0, 1, heart.x, heart.y);
      ctx.rotate(heart.angle + Math.PI / 2);
      ctx.drawImage(this.heartSprite, -half, -half, size, size);
    }

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.globalAlpha = 1;
    ctx.globalCompositeOperation = "source-over";
    return true;
  }
}

import type { MouseTrailEngine } from "./mouse-trail-types";

export interface DotsTrailOptions {
  color?: string;
  /** Squared max link distance (mouser options.number). Default 8000. */
  linkDistanceSq?: number;
  count?: number;
}

interface Dot {
  x: number;
  y: number;
  xa: number;
  ya: number;
  max: number;
}

interface MouseAnchor {
  x: number | null;
  y: number | null;
  max: number;
}

function hexToRgba(hex: string, opacity: number): string {
  const cleaned = hex.replace(/^#/, "");
  if (cleaned.length !== 6) {
    return `rgba(165,251,255,${opacity})`;
  }
  const r = parseInt(cleaned.slice(0, 2), 16);
  const g = parseInt(cleaned.slice(2, 4), 16);
  const b = parseInt(cleaned.slice(4, 6), 16);
  return `rgba(${r},${g},${b},${opacity})`;
}

/**
 * Connected floating dots (mouser-lqy drawType 5).
 */
export class DotsTrail implements MouseTrailEngine {
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly color: string;
  private readonly linkDistanceSq: number;
  private readonly count: number;
  private dots: Dot[] = [];
  private readonly mouse: MouseAnchor;
  private rafId = 0;
  private running = false;

  constructor(host: HTMLElement, options: DotsTrailOptions = {}) {
    this.color = options.color ?? "#A5FBFF";
    this.linkDistanceSq = options.linkDistanceSq ?? 8000;
    this.count = Math.max(40, options.count ?? 300);
    this.mouse = { x: null, y: null, max: this.linkDistanceSq };

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
    this.seedDots();
    window.addEventListener("resize", this.onResize);
  }

  private onResize = () => {
    this.resize();
  };

  private seedDots() {
    const { width, height } = this.canvas;
    this.dots = [];
    for (let i = 0; i < this.count; i += 1) {
      this.dots.push({
        x: Math.random() * width,
        y: Math.random() * height,
        xa: Math.random() * 2 - 1,
        ya: Math.random() * 2 - 1,
        max: this.linkDistanceSq,
      });
    }
  }

  resize() {
    const width = this.canvas.clientWidth || window.innerWidth;
    const height = this.canvas.clientHeight || window.innerHeight;
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
      for (const dot of this.dots) {
        dot.x = Math.min(Math.max(dot.x, 0), width);
        dot.y = Math.min(Math.max(dot.y, 0), height);
      }
      if (this.dots.length === 0) {
        this.seedDots();
      }
    }
  }

  setMouse(x: number, y: number) {
    this.mouse.x = x;
    this.mouse.y = y;
    this.kick();
  }

  leaveScreen() {
    this.mouse.x = null;
    this.mouse.y = null;
  }

  start() {
    this.kick();
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
      this.render();
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
    window.removeEventListener("resize", this.onResize);
    this.canvas.remove();
  }

  private render() {
    const ctx = this.ctx;
    const width = this.canvas.width;
    const height = this.canvas.height;
    ctx.clearRect(0, 0, width, height);

    const ndots: Array<Dot | MouseAnchor> = [this.mouse, ...this.dots];
    for (const dot of this.dots) {
      dot.x += dot.xa;
      dot.y += dot.ya;
      if (dot.x > width || dot.x < 0) {
        dot.xa *= -1;
      }
      if (dot.y > height || dot.y < 0) {
        dot.ya *= -1;
      }
      ctx.fillStyle = this.color;
      ctx.fillRect(dot.x - 0.5, dot.y - 0.5, 1, 1);

      for (let i = 0; i < ndots.length; i += 1) {
        const other = ndots[i];
        if (dot === other || other.x === null || other.y === null) {
          continue;
        }
        const xc = dot.x - other.x;
        const yc = dot.y - other.y;
        const dis = xc * xc + yc * yc;
        if (dis >= other.max) {
          continue;
        }
        if (other === this.mouse && dis > other.max / 2) {
          dot.x -= xc * 0.03;
          dot.y -= yc * 0.03;
        }
        const ratio = (other.max - dis) / other.max;
        ctx.beginPath();
        ctx.lineWidth = ratio / 2;
        ctx.strokeStyle = hexToRgba(this.color, ratio + 0.2);
        ctx.moveTo(dot.x, dot.y);
        ctx.lineTo(other.x, other.y);
        ctx.stroke();
      }
      const idx = ndots.indexOf(dot);
      if (idx >= 0) {
        ndots.splice(idx, 1);
      }
    }
  }
}

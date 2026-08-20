import type { MouseTrailEngine } from "./mouse-trail-types";

export interface RibbonTrailOptions {
  showTime?: number;
  maxWidth?: number;
  minWidth?: number;
}

interface Point {
  x: number;
  y: number;
}

/**
 * Rainbow ribbon trail (mouser-lqy drawType 1), without leaveAutoer heart path.
 */
export class RibbonTrail implements MouseTrailEngine {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private body: Point[] = [];
  private red: number[] = [];
  private grn: number[] = [];
  private blu: number[] = [];
  private mouseX = 0;
  private mouseY = 0;
  private line = 0;
  private lineOp = 1;
  private step = 0;
  private loop = 0;
  private running = false;
  private rafId = 0;
  private onScreen = false;
  /** After leave/cold start, next setMouse seeds the whole trail at the entry point. */
  private needsReseed = true;
  private readonly bodyLength: number;
  private readonly lineMax: number;
  private readonly lineMin: number;

  constructor(host: HTMLElement, options: RibbonTrailOptions = {}) {
    this.bodyLength = Math.max(8, options.showTime ?? 20);
    this.lineMax = options.maxWidth ?? 12;
    this.lineMin = options.minWidth ?? 4;
    this.line = this.lineMin + 1;

    this.canvas = document.createElement("canvas");
    this.canvas.style.cssText =
      "position:absolute;inset:0;width:100%;height:100%;pointer-events:none;";
    host.appendChild(this.canvas);
    const ctx = this.canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Canvas 2D unavailable");
    }
    this.ctx = ctx;

    const center = 128;
    const width = 127;
    for (let s = 0; s < this.bodyLength; s += 1) {
      this.red[s] = Math.round(Math.sin(0.3 * s) * width + center);
      this.grn[s] = Math.round(Math.sin(0.3 * s + 2) * width + center);
      this.blu[s] = Math.round(Math.sin(0.3 * s + 4) * width + center);
    }

    this.resize();
    this.body = Array.from({ length: this.bodyLength }, () => ({ x: 0, y: 0 }));
    this.needsReseed = true;
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

  /** Collapse the whole ribbon onto a single point (entry / reseed). */
  private seedBody(x: number, y: number) {
    this.mouseX = x;
    this.mouseY = y;
    this.body = Array.from({ length: this.bodyLength }, () => ({ x, y }));
  }

  setMouse(x: number, y: number) {
    if (this.canvas.width === 0 || this.canvas.height === 0) {
      this.resize();
    }
    if (this.needsReseed) {
      this.seedBody(x, y);
      this.needsReseed = false;
    }
    this.onScreen = true;
    this.mouseX = x;
    this.mouseY = y;
    this.kick();
  }

  leaveScreen() {
    this.onScreen = false;
    this.needsReseed = true;
    this.stopDrawing(true);
  }

  start() {
    // Idle until first setMouse.
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
      if (!this.onScreen) {
        this.stopDrawing(true);
        return;
      }
      this.render();
      this.rafId = window.requestAnimationFrame(tick);
    };
    this.rafId = window.requestAnimationFrame(tick);
  }

  private stopDrawing(clear: boolean) {
    this.running = false;
    if (this.rafId) {
      window.cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
    if (clear) {
      this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    }
  }

  stop() {
    this.onScreen = false;
    this.needsReseed = true;
    this.stopDrawing(true);
  }

  destroy() {
    this.stop();
    window.removeEventListener("resize", this.onResize);
    this.canvas.remove();
  }

  private render() {
    const ctx = this.ctx;
    const twoPi = Math.PI * 2;

    if (this.line <= this.lineMin) {
      this.lineOp = 1;
      this.line = this.lineMin + 1;
    }
    if (this.line >= this.lineMax) {
      this.lineOp = -1;
      this.line = this.lineMax - 1;
    }
    this.loop += 1;
    if (this.loop === 5) {
      this.step = (this.step + 1) % this.bodyLength;
      this.loop = 0;
    }
    this.line += this.lineOp;

    for (let i = this.body.length - 1; i > 0; i -= 1) {
      this.body[i].x = this.body[i - 1].x;
      this.body[i].y = this.body[i - 1].y;
    }
    this.body[0].x = this.mouseX;
    this.body[0].y = this.mouseY;

    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    const color = `rgb(${this.red[this.step]},${this.grn[this.step]},${this.blu[this.step]})`;
    ctx.lineWidth = this.line;
    ctx.strokeStyle = color;
    ctx.fillStyle = color;

    ctx.beginPath();
    ctx.arc(this.body[0].x, this.body[0].y, this.line / 2, 0, twoPi);
    ctx.fill();

    ctx.beginPath();
    ctx.moveTo(this.body[0].x, this.body[0].y);
    let xc = this.body[0].x;
    let yc = this.body[0].y;
    for (let s = 0; s < this.body.length - 2; s += 1) {
      xc = (this.body[s].x + this.body[s + 1].x) / 2;
      yc = (this.body[s].y + this.body[s + 1].y) / 2;
      ctx.quadraticCurveTo(this.body[s].x, this.body[s].y, xc, yc);
    }
    ctx.stroke();

    ctx.beginPath();
    ctx.arc(xc, yc, this.line / 2, 0, twoPi);
    ctx.fill();
  }
}

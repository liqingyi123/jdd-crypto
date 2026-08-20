export interface MeteorTrailOptions {
  color?: string;
}

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  scale: number;
  angle: number;
}

interface MouseTracker {
  x: number;
  y: number;
  speedX: number;
  speedY: number;
  update(x: number, y: number): void;
}

const MAX_PARTICLES = 400;

function parseColor(input: string): ["rgba", number, number, number, number] | null {
  const hex = input.replace(/^\s*#|\s*$/g, "").toLowerCase();
  if (/^([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/.test(hex)) {
    return [
      "rgba",
      parseInt(hex.slice(0, 2), 16),
      parseInt(hex.slice(2, 4), 16),
      parseInt(hex.slice(4, 6), 16),
      1,
    ];
  }
  const rgb = input.match(/^rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)$/);
  if (rgb) {
    return ["rgba", Number(rgb[1]), Number(rgb[2]), Number(rgb[3]), 1];
  }
  return null;
}

function createStarCanvas(color: string, size: number, blur: number): HTMLCanvasElement {
  const parsed = parseColor(color) ?? (["rgba", 248, 236, 133, 1] as const);
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return canvas;
  }
  const center = size / 2;
  const inner = size * 0.1;
  const outer = size * 0.5;
  ctx.fillStyle = `rgba(${parsed[1]},${parsed[2]},${parsed[3]},${parsed[4]})`;
  ctx.shadowBlur = blur;
  ctx.shadowColor = `rgba(${parsed[1]},${parsed[2]},${parsed[3]},0.75)`;
  ctx.beginPath();
  for (let point = 1; point <= 10; point += 1) {
    const radius = point % 2 === 1 ? inner : outer;
    const angle = (Math.PI * 2 * point) / 10;
    const x = center + radius * Math.cos(angle);
    const y = center + radius * Math.sin(angle);
    if (point === 1) {
      ctx.moveTo(x, y);
    } else {
      ctx.lineTo(x, y);
    }
  }
  ctx.fill();
  return canvas;
}

export class MeteorTrail {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private color: string;
  private mouse: MouseTracker;
  private particles: Particle[] = [];
  private pool: Particle[] = [];
  private symbols: HTMLCanvasElement[] = [];
  private symbolSize = 0;
  private running = false;
  private rafId = 0;
  private onScreen = false;
  private spawning = false;

  constructor(host: HTMLElement, options: MeteorTrailOptions = {}) {
    this.color = options.color ?? "#F8EC85";
    this.canvas = document.createElement("canvas");
    this.canvas.style.cssText =
      "position:absolute;inset:0;width:100%;height:100%;pointer-events:none;";
    host.appendChild(this.canvas);
    const ctx = this.canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Canvas 2D unavailable");
    }
    this.ctx = ctx;
    this.mouse = {
      x: 0,
      y: 0,
      speedX: 0,
      speedY: 0,
      update(x: number, y: number) {
        this.speedX = (this.x - x) * 0.7;
        this.speedY = (this.y - y) * 0.7;
        this.x = x;
        this.y = y;
      },
    };
    this.rebuildSymbols();
    this.resize();
    window.addEventListener("resize", this.onResize);
  }

  private onResize = () => {
    this.resize();
  };

  private rebuildSymbols() {
    this.symbolSize = 28;
    this.symbols = [4, 6, 8, 10, 12].map(() =>
      createStarCanvas(this.color, this.symbolSize, 14),
    );
  }

  setColor(color: string) {
    if (this.color === color) {
      return;
    }
    this.color = color;
    this.rebuildSymbols();
  }

  resize() {
    const width = this.canvas.clientWidth || window.innerWidth;
    const height = this.canvas.clientHeight || window.innerHeight;
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
  }

  setMouse(x: number, y: number) {
    this.onScreen = true;
    this.spawning = true;
    this.mouse.update(x, y);
    this.kick();
  }

  /** Cursor left this monitor: stop spawning, let particles fade out. */
  leaveScreen() {
    if (!this.onScreen && !this.spawning) {
      return;
    }
    this.onScreen = false;
    this.spawning = false;
    this.mouse.speedX = 0;
    this.mouse.speedY = 0;
    if (this.particles.length > 0) {
      this.kick();
    }
  }

  start() {
    // Idle until first setMouse; kick starts the loop.
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
      const keepGoing = this.render();
      if (!keepGoing) {
        this.running = false;
        this.rafId = 0;
        return;
      }
      this.rafId = window.requestAnimationFrame(tick);
    };
    this.rafId = window.requestAnimationFrame(tick);
  }

  stop() {
    this.running = false;
    this.onScreen = false;
    this.spawning = false;
    if (this.rafId) {
      window.cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
    this.particles = [];
    this.pool = [];
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  destroy() {
    this.stop();
    window.removeEventListener("resize", this.onResize);
    this.canvas.remove();
  }

  private spawnParticle(minSpeed: number, maxSpeed: number, scaleBase: number) {
    const speed = minSpeed + (maxSpeed - minSpeed) * Math.random();
    const angle = Math.random() * Math.PI * 2;
    const particle =
      this.pool.length > 0
        ? this.pool.pop()!
        : { x: 0, y: 0, vx: 0, vy: 0, scale: 0, angle: 0 };
    particle.x = this.mouse.x;
    particle.y = this.mouse.y;
    particle.vx = speed * Math.cos(angle);
    particle.vy = speed * Math.sin(angle);
    particle.scale = scaleBase * Math.random();
    particle.angle = Math.random() * Math.PI * 2;
    this.particles.push(particle);
  }

  /** @returns whether another frame is needed */
  private render(): boolean {
    const ctx = this.ctx;
    if (!this.spawning && this.particles.length === 0) {
      ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
      return false;
    }

    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    const speed = Math.min(
      0.005 * (this.mouse.speedX ** 2 + this.mouse.speedY ** 2),
      1,
    );

    if (this.spawning && this.particles.length < MAX_PARTICLES) {
      const burst = 2 + ((12 * speed) | 0);
      for (let i = 0; i < burst; i += 1) {
        this.spawnParticle(0.1 + 0.5 * speed, 0.5 + 4.5 * speed, 0.5 + 0.5 * speed);
      }
    }

    const margin = this.symbolSize;
    const minX = -margin;
    const maxX = this.canvas.width + margin;
    const minY = -margin;
    const maxY = this.canvas.height + margin;

    ctx.globalCompositeOperation = "lighter";
    for (let i = this.particles.length - 1; i >= 0; i -= 1) {
      const particle = this.particles[i];
      particle.vx += this.mouse.speedX * speed * 0.03;
      particle.vy += this.mouse.speedY * speed * 0.03 + 0.035;
      particle.x += particle.vx + this.mouse.speedX;
      particle.y += particle.vy + this.mouse.speedY;
      particle.scale -= 0.01;
      particle.angle += 0.2;

      if (
        particle.x + margin < minX ||
        particle.x - margin > maxX ||
        particle.y + margin < minY ||
        particle.y - margin > maxY ||
        particle.scale <= 0
      ) {
        this.pool.push(particle);
        this.particles.splice(i, 1);
        continue;
      }

      const symbol =
        this.symbols[(this.symbols.length * Math.random()) | 0] ?? this.symbols[0];
      let drawScale = particle.scale;
      if (Math.random() < 0.7) {
        drawScale *= 0.2;
      }
      const size = this.symbolSize * drawScale;
      const half = size * 0.5;
      ctx.setTransform(1, 0, 0, 1, particle.x, particle.y);
      ctx.rotate(particle.angle);
      ctx.drawImage(symbol, -half, -half, size, size);
    }
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.globalCompositeOperation = "source-over";

    return this.spawning || this.particles.length > 0;
  }
}

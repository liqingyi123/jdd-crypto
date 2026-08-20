import type { MouseTrailEngine } from "./mouse-trail-types";

const SVG_NS = "http://www.w3.org/2000/svg";
const COLORS = ["red", "blue", "green", "yellow", "white"] as const;
const REMOVE_DELAY_MS = 400;
const SHAPE_CHANCE = 0.1;
const MIN_SHAPE_SIZE = 1.3;

interface Vec2 {
  x: number;
  y: number;
}

interface TrailPoint {
  position: Vec2;
  time: number;
  drift: Vec2;
  age: number;
  direction: Vec2;
}

interface ShapeAnim {
  rafId: number;
  shape: SVGElement;
}

/**
 * Street-graffiti SVG trail (mouser-lqy drawType 4), without TweenMax/Rx.
 */
class GraffitiFollower {
  private readonly stage: SVGSVGElement;
  private readonly color: string;
  private readonly line: SVGPathElement;
  private readonly points: TrailPoint[] = [];
  private readonly shapeAnims = new Set<ShapeAnim>();
  private rafId = 0;
  private alive = true;

  constructor(stage: SVGSVGElement, color: string) {
    this.stage = stage;
    this.color = color;
    this.line = document.createElementNS(SVG_NS, "path");
    this.line.style.fill = color;
    stage.appendChild(this.line);
    this.tick();
  }

  add(position: Vec2) {
    const direction: Vec2 = { x: 0, y: 0 };
    const head = this.points[0];
    if (head) {
      direction.x = (position.x - head.position.x) * 0.25;
      direction.y = (position.y - head.position.y) * 0.25;
    }
    const point: TrailPoint = {
      position: { ...position },
      time: Date.now(),
      drift: {
        x: this.getDrift() + direction.x / 2,
        y: this.getDrift() + direction.y / 2,
      },
      age: 0,
      direction,
    };
    const shapeChance = Math.random();
    if (shapeChance < SHAPE_CHANCE) {
      this.makeCircle(point);
    } else if (shapeChance < SHAPE_CHANCE * 2) {
      this.makeSquare(point);
    } else if (shapeChance < SHAPE_CHANCE * 3) {
      this.makeTriangle(point);
    }
    this.points.unshift(point);
  }

  clear() {
    this.points.length = 0;
    this.line.setAttribute("d", "");
    this.cancelShapeAnims(true);
  }

  destroy() {
    this.alive = false;
    if (this.rafId) {
      window.cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
    this.cancelShapeAnims(true);
    this.line.remove();
  }

  private cancelShapeAnims(removeShapes: boolean) {
    for (const anim of this.shapeAnims) {
      window.cancelAnimationFrame(anim.rafId);
      if (removeShapes) {
        anim.shape.remove();
      }
    }
    this.shapeAnims.clear();
  }

  private getDrift() {
    return (Math.random() - 0.5) * 3;
  }

  private shapeSize(point: TrailPoint, scale = 1) {
    return Math.max(
      MIN_SHAPE_SIZE,
      ((Math.abs(point.direction.x) + Math.abs(point.direction.y)) * scale) / 3,
    );
  }

  private createLine(points: TrailPoint[]): string {
    if (points.length === 0) {
      return "";
    }
    const path: string[] = ["M"];
    let forward = true;
    let i = 0;
    while (i >= 0) {
      const point = points[i];
      const offsetX = point.direction.x * ((i - points.length) / points.length) * 0.3;
      const offsetY = point.direction.y * ((i - points.length) / points.length) * 0.3;
      const x = point.position.x + (forward ? offsetY : -offsetY);
      const y = point.position.y + (forward ? offsetX : -offsetX);
      point.age += 0.2;
      path.push(String(x + point.drift.x * point.age));
      path.push(String(y + point.drift.y * point.age));
      i += forward ? 1 : -1;
      if (i === points.length) {
        i -= 1;
        forward = false;
      }
    }
    return path.join(" ");
  }

  private tick = () => {
    if (!this.alive) {
      return;
    }
    if (this.points.length > 0) {
      const last = this.points[this.points.length - 1];
      if (last.time < Date.now() - REMOVE_DELAY_MS) {
        this.points.pop();
      }
    }
    this.line.setAttribute("d", this.createLine(this.points));
    this.rafId = window.requestAnimationFrame(this.tick);
  };

  private makeCircle(point: TrailPoint) {
    const circle = document.createElementNS(SVG_NS, "circle");
    circle.setAttribute("r", String(this.shapeSize(point, 1)));
    circle.setAttribute("fill", this.color);
    circle.setAttribute("cx", "0");
    circle.setAttribute("cy", "0");
    this.moveShape(circle, point);
  }

  private makeSquare(point: TrailPoint) {
    const size = this.shapeSize(point, 1.5);
    const square = document.createElementNS(SVG_NS, "rect");
    square.setAttribute("width", String(size));
    square.setAttribute("height", String(size));
    square.setAttribute("fill", this.color);
    this.moveShape(square, point);
  }

  private makeTriangle(point: TrailPoint) {
    const size = this.shapeSize(point, 1.5);
    const triangle = document.createElementNS(SVG_NS, "polygon");
    triangle.setAttribute("points", `0,0 ${size},${size / 2} 0,${size}`);
    triangle.setAttribute("fill", this.color);
    this.moveShape(triangle, point);
  }

  /** SVG attribute transform + RAF (WebView-safe; CSS transform on SVG is unreliable). */
  private moveShape(shape: SVGElement, point: TrailPoint) {
    this.stage.appendChild(shape);
    const startX = point.position.x;
    const startY = point.position.y;
    const endX =
      point.position.x +
      point.direction.x * (Math.random() * 20) +
      point.drift.x * (Math.random() * 10);
    const endY =
      point.position.y +
      point.direction.y * (Math.random() * 20) +
      point.drift.y * (Math.random() * 10);
    const durationMs = (0.5 + Math.random()) * 1000;
    const endRot = Math.random() * 360;

    const apply = (x: number, y: number, scale: number, rot: number) => {
      shape.setAttribute(
        "transform",
        `translate(${x} ${y}) rotate(${rot}) scale(${scale})`,
      );
    };

    apply(startX, startY, 1, 0);

    const anim: ShapeAnim = { rafId: 0, shape };
    let startTime = 0;

    const step = (now: number) => {
      if (!this.alive || !this.shapeAnims.has(anim)) {
        return;
      }
      if (!startTime) {
        startTime = now;
        // Hold first visible frame, then animate on next tick.
        anim.rafId = window.requestAnimationFrame(step);
        return;
      }
      const t = Math.min(1, (now - startTime) / durationMs);
      // Power4.easeOut-ish
      const eased = 1 - (1 - t) ** 4;
      apply(
        startX + (endX - startX) * eased,
        startY + (endY - startY) * eased,
        1 - eased,
        endRot * eased,
      );
      if (t < 1) {
        anim.rafId = window.requestAnimationFrame(step);
        return;
      }
      this.shapeAnims.delete(anim);
      shape.remove();
    };

    this.shapeAnims.add(anim);
    anim.rafId = window.requestAnimationFrame(step);
  }
}

/**
 * Street graffiti trail adapted from mouser-lqy drawType 4.
 */
export class GraffitiTrail implements MouseTrailEngine {
  private readonly host: HTMLElement;
  private readonly svg: SVGSVGElement;
  private readonly followers: GraffitiFollower[] = [];
  private onScreen = false;
  private lastPos: Vec2 | null = null;

  constructor(host: HTMLElement) {
    this.host = host;
    this.svg = document.createElementNS(SVG_NS, "svg");
    this.svg.setAttribute("xmlns", SVG_NS);
    this.svg.style.cssText =
      "position:absolute;inset:0;width:100%;height:100%;pointer-events:none;overflow:visible;";
    host.appendChild(this.svg);
    for (const color of COLORS) {
      this.followers.push(new GraffitiFollower(this.svg, color));
    }
    this.resize();
    window.addEventListener("resize", this.onResize);
  }

  private onResize = () => {
    this.resize();
  };

  resize() {
    const width = this.host.clientWidth || window.innerWidth;
    const height = this.host.clientHeight || window.innerHeight;
    this.svg.setAttribute("width", String(width));
    this.svg.setAttribute("height", String(height));
    this.svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  }

  setMouse(x: number, y: number) {
    this.onScreen = true;
    const pos = { x, y };
    if (
      this.lastPos &&
      Math.abs(this.lastPos.x - x) < 0.5 &&
      Math.abs(this.lastPos.y - y) < 0.5
    ) {
      return;
    }
    this.lastPos = pos;
    for (const follower of this.followers) {
      follower.add(pos);
    }
  }

  leaveScreen() {
    this.onScreen = false;
    this.lastPos = null;
    for (const follower of this.followers) {
      follower.clear();
    }
  }

  start() {
    // Idle until setMouse.
  }

  stop() {
    this.onScreen = false;
    this.lastPos = null;
    for (const follower of this.followers) {
      follower.clear();
    }
  }

  destroy() {
    window.removeEventListener("resize", this.onResize);
    for (const follower of this.followers) {
      follower.destroy();
    }
    this.followers.length = 0;
    this.svg.remove();
    void this.onScreen;
  }
}

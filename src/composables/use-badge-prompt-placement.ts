import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  readonly,
  shallowRef,
  toValue,
  watch,
  type MaybeRefOrGetter,
} from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  EXPANDED_EXTRA_HEIGHT,
  EXPANDED_EXTRA_WIDTH,
} from "@/constants/badge";

/** Must match `EXPANDED_EXTRA_WIDTH` in src-tauri/src/windows.rs */
const EXTRA_W = EXPANDED_EXTRA_WIDTH;
/** Must match `EXPANDED_EXTRA_HEIGHT` in src-tauri/src/windows.rs */
const EXTRA_H = EXPANDED_EXTRA_HEIGHT;

const PLACEMENT_ORDER = ["right", "left"] as const;

export type PromptPlacement = (typeof PLACEMENT_ORDER)[number];
type VerticalAlign = "down" | "up";

interface Point {
  x: number;
  y: number;
}

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface MonitorWorkArea {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactor: number;
}

export function useBadgePromptPlacement(options: {
  promptOpen: MaybeRefOrGetter<boolean>;
  badgeSize: MaybeRefOrGetter<number>;
}) {
  const placement = shallowRef<PromptPlacement>("right");
  const vAlign = shallowRef<VerticalAlign>("down");
  const placementClass = computed(() => {
    const sideClass = `prompt-${placement.value}`;
    return vAlign.value === "up" ? `${sideClass} prompt-up` : sideClass;
  });

  let monitors: MonitorWorkArea[] = [];
  let cachedOrb: Point | null = null;
  let lastScale = 1;
  let lastPos: Point | null = null;
  let lastLogicalSize: Point | null = null;
  let rafId = 0;
  let pendingOrb: Point | null = null;
  let applying = false;
  let queuedOrb: Point | null = null;
  let disposed = false;
  let dragStartScreen: Point | null = null;
  let dragStartOrb: Point | null = null;
  let dragScale = 1;

  function logicalWindowSize(size: number): { w: number; h: number } {
    return { w: size + EXTRA_W, h: size + EXTRA_H };
  }

  function windowRect(
    side: PromptPlacement,
    align: VerticalAlign,
    orb: Point,
    size: number,
    scale: number,
  ): Rect {
    const extraW = EXTRA_W * scale;
    const extraH = EXTRA_H * scale;
    const sizePx = size * scale;
    return {
      x: side === "left" ? orb.x - extraW : orb.x,
      y: align === "up" ? orb.y - extraH : orb.y,
      w: sizePx + extraW,
      h: sizePx + extraH,
    };
  }

  function overflowAmount(rect: Rect, area: MonitorWorkArea): number {
    const left = Math.max(0, area.x - rect.x);
    const top = Math.max(0, area.y - rect.y);
    const right = Math.max(0, rect.x + rect.w - (area.x + area.width));
    const bottom = Math.max(0, rect.y + rect.h - (area.y + area.height));
    return left + top + right + bottom;
  }

  function clampRect(rect: Rect, area: MonitorWorkArea): Rect {
    let x = rect.x;
    let y = rect.y;
    if (rect.w <= area.width) {
      x = Math.min(Math.max(rect.x, area.x), area.x + area.width - rect.w);
    } else {
      x = area.x;
    }
    if (rect.h <= area.height) {
      y = Math.min(Math.max(rect.y, area.y), area.y + area.height - rect.h);
    } else {
      y = area.y;
    }
    return { ...rect, x, y };
  }

  function pickVAlign(
    orb: Point,
    size: number,
    scale: number,
    area: MonitorWorkArea,
    current: VerticalAlign,
  ): VerticalAlign {
    const extraH = EXTRA_H * scale;
    const sizePx = size * scale;
    const spaceBelow = area.y + area.height - (orb.y + sizePx);
    const spaceAbove = orb.y - area.y;
    if (current === "down" && spaceBelow >= extraH) {
      return "down";
    }
    if (current === "up" && spaceAbove >= extraH) {
      return "up";
    }
    return spaceBelow >= extraH ? "down" : "up";
  }

  function pickPlacement(
    orb: Point,
    size: number,
    scale: number,
    area: MonitorWorkArea,
    align: VerticalAlign,
    current: PromptPlacement,
  ): PromptPlacement {
    if (overflowAmount(windowRect(current, align, orb, size, scale), area) === 0) {
      return current;
    }
    for (const side of PLACEMENT_ORDER) {
      if (overflowAmount(windowRect(side, align, orb, size, scale), area) === 0) {
        return side;
      }
    }
    let best: PromptPlacement = "right";
    let bestOverflow = Number.POSITIVE_INFINITY;
    for (const side of PLACEMENT_ORDER) {
      const amount = overflowAmount(
        windowRect(side, align, orb, size, scale),
        area,
      );
      if (amount < bestOverflow) {
        bestOverflow = amount;
        best = side;
      }
    }
    return best;
  }

  function workAreaFor(orb: Point): MonitorWorkArea | null {
    for (const area of monitors) {
      if (
        orb.x >= area.x &&
        orb.x < area.x + area.width &&
        orb.y >= area.y &&
        orb.y < area.y + area.height
      ) {
        return area;
      }
    }
    return null;
  }

  async function loadWindowApi() {
    return import("@tauri-apps/api/window");
  }

  async function refreshMonitors(): Promise<void> {
    try {
      const { availableMonitors } = await loadWindowApi();
      const list = await availableMonitors();
      monitors = list.map((monitor) => ({
        x: monitor.workArea.position.x,
        y: monitor.workArea.position.y,
        width: monitor.workArea.size.width,
        height: monitor.workArea.size.height,
        scaleFactor: monitor.scaleFactor,
      }));
    } catch {
      monitors = [];
    }
  }

  async function resolveWorkArea(orb: Point): Promise<MonitorWorkArea | null> {
    let area = workAreaFor(orb);
    if (area) {
      return area;
    }
    await refreshMonitors();
    area = workAreaFor(orb);
    if (area) {
      return area;
    }
    return monitors[0] ?? null;
  }

  async function readWindowOrigin(): Promise<{ origin: Point; scale: number } | null> {
    try {
      const { getCurrentWindow } = await loadWindowApi();
      const win = getCurrentWindow();
      const [pos, scale] = await Promise.all([
        win.outerPosition(),
        win.scaleFactor(),
      ]);
      return { origin: { x: pos.x, y: pos.y }, scale };
    } catch {
      return null;
    }
  }

  function orbFromOrigin(
    origin: Point,
    expanded: boolean,
    side: PromptPlacement,
    align: VerticalAlign,
    scale: number,
  ): Point {
    if (!expanded) {
      return origin;
    }
    return {
      x: side === "left" ? origin.x + EXTRA_W * scale : origin.x,
      y: align === "up" ? origin.y + EXTRA_H * scale : origin.y,
    };
  }

  async function applyGeometry(orb: Point): Promise<void> {
    if (disposed) {
      return;
    }
    if (applying) {
      queuedOrb = orb;
      return;
    }
    applying = true;
    try {
      const size = toValue(options.badgeSize);
      const area = await resolveWorkArea(orb);
      if (!area) {
        return;
      }
      lastScale = area.scaleFactor;
      const align = pickVAlign(
        orb,
        size,
        area.scaleFactor,
        area,
        vAlign.value,
      );
      const side = pickPlacement(
        orb,
        size,
        area.scaleFactor,
        area,
        align,
        placement.value,
      );
      const rect = clampRect(
        windowRect(side, align, orb, size, area.scaleFactor),
        area,
      );
      const logical = logicalWindowSize(size);
      const nextPos = { x: Math.round(rect.x), y: Math.round(rect.y) };
      const posChanged = !lastPos || lastPos.x !== nextPos.x || lastPos.y !== nextPos.y;
      const sizeChanged =
        !lastLogicalSize ||
        lastLogicalSize.x !== logical.w ||
        lastLogicalSize.y !== logical.h;

      if (placement.value !== side) {
        placement.value = side;
      }
      if (vAlign.value !== align) {
        vAlign.value = align;
      }

      if (!posChanged && !sizeChanged) {
        return;
      }

      const { getCurrentWindow, LogicalSize, PhysicalPosition } =
        await loadWindowApi();
      const win = getCurrentWindow();
      const tasks: Promise<void>[] = [];
      if (sizeChanged) {
        tasks.push(win.setSize(new LogicalSize(logical.w, logical.h)));
      }
      if (posChanged) {
        tasks.push(
          win.setPosition(new PhysicalPosition(nextPos.x, nextPos.y)),
        );
      }
      await Promise.all(tasks);
      lastPos = nextPos;
      lastLogicalSize = { x: logical.w, y: logical.h };
    } catch {
      // browser preview
    } finally {
      applying = false;
      if (queuedOrb) {
        const next = queuedOrb;
        queuedOrb = null;
        await applyGeometry(next);
      }
    }
  }

  function scheduleApply(orb: Point): void {
    cachedOrb = orb;
    pendingOrb = orb;
    if (rafId) {
      return;
    }
    rafId = requestAnimationFrame(() => {
      rafId = 0;
      const next = pendingOrb;
      pendingOrb = null;
      if (next) {
        void applyGeometry(next);
      }
    });
  }

  async function syncOpen(): Promise<void> {
    const snapshot = await readWindowOrigin();
    if (snapshot) {
      cachedOrb = orbFromOrigin(
        snapshot.origin,
        false,
        "right",
        "down",
        snapshot.scale,
      );
      lastScale = snapshot.scale;
    }
    await refreshMonitors();
    await invoke("set_badge_prompt_mode", { expanded: true }).catch(
      () => undefined,
    );
    if (cachedOrb) {
      await applyGeometry(cachedOrb);
    }
  }

  async function syncClose(): Promise<void> {
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
    pendingOrb = null;
    queuedOrb = null;

    const closingPlacement = placement.value;
    const closingAlign = vAlign.value;
    placement.value = "right";
    vAlign.value = "down";

    let orb = cachedOrb;
    if (!orb) {
      const snapshot = await readWindowOrigin();
      if (snapshot) {
        orb = orbFromOrigin(
          snapshot.origin,
          true,
          closingPlacement,
          closingAlign,
          snapshot.scale,
        );
      }
    }

    await nextTick();

    if (orb) {
      try {
        const { getCurrentWindow, PhysicalPosition } = await loadWindowApi();
        await getCurrentWindow().setPosition(
          new PhysicalPosition(Math.round(orb.x), Math.round(orb.y)),
        );
        lastPos = { x: Math.round(orb.x), y: Math.round(orb.y) };
      } catch {
        // browser preview
      }
      cachedOrb = orb;
    }

    const size = toValue(options.badgeSize);
    lastLogicalSize = { x: size, y: size };
    await invoke("set_badge_prompt_mode", { expanded: false }).catch(
      () => undefined,
    );
  }

  function beginExpandedDrag(event: PointerEvent): void {
    dragStartScreen = { x: event.screenX, y: event.screenY };
    dragStartOrb = cachedOrb ? { ...cachedOrb } : null;
    dragScale = lastScale || 1;
  }

  function moveExpandedDrag(event: PointerEvent): void {
    if (!dragStartScreen || !dragStartOrb) {
      return;
    }
    scheduleApply({
      x: dragStartOrb.x + (event.screenX - dragStartScreen.x) * dragScale,
      y: dragStartOrb.y + (event.screenY - dragStartScreen.y) * dragScale,
    });
  }

  function endExpandedDrag(): void {
    dragStartScreen = null;
    dragStartOrb = null;
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
    const orb = pendingOrb ?? cachedOrb;
    pendingOrb = null;
    if (orb) {
      void applyGeometry(orb);
    }
  }

  watch(
    () => toValue(options.promptOpen),
    async (open) => {
      if (open) {
        await syncOpen();
        return;
      }
      await syncClose();
    },
  );

  watch(
    () => toValue(options.badgeSize),
    () => {
      if (!toValue(options.promptOpen) || !cachedOrb) {
        return;
      }
      void applyGeometry(cachedOrb);
    },
  );

  onMounted(async () => {
    const snapshot = await readWindowOrigin();
    if (snapshot) {
      cachedOrb = snapshot.origin;
      lastScale = snapshot.scale;
      lastPos = snapshot.origin;
    }
  });

  onUnmounted(() => {
    disposed = true;
    if (rafId) {
      cancelAnimationFrame(rafId);
    }
  });

  return {
    placement: readonly(placement),
    placementClass,
    beginExpandedDrag,
    moveExpandedDrag,
    endExpandedDrag,
  };
}

import type { MouseTrailPerformanceMode } from "./mouse-trail-types";

/** Draw / effect FPS. 0 = uncapped (follow display refresh). */
export function targetFpsForMode(mode: MouseTrailPerformanceMode): number {
  if (mode === "eco" || mode === "balanced") {
    return 60;
  }
  return 0;
}

/** Cursor peek FPS (aligned with Rust sample rate). 0 = uncapped. */
export function peekFpsForMode(mode: MouseTrailPerformanceMode): number {
  if (mode === "eco") {
    return 30;
  }
  if (mode === "balanced") {
    return 60;
  }
  return 0;
}

export interface FrameGate {
  setTargetFps(fps: number): void;
  /** Whether work should run this vsync tick. Always true when uncapped. */
  shouldRun(now: number): boolean;
}

/**
 * Time-gated frame budget for rAF loops. Keeps scheduling every vsync but
 * skips work until the target interval elapses; allows mild catch-up so the
 * gate does not drift forever behind.
 *
 * Multiple callers in the same rAF timestamp share one decision so a shared
 * gate (e.g. graffiti followers + shape anims) does not starve siblings.
 */
export function createFrameGate(initialFps = 0): FrameGate {
  let targetFps = initialFps;
  let lastRun = 0;
  let decisionNow = -1;
  let decisionAllow = false;

  return {
    setTargetFps(fps: number) {
      targetFps = fps;
      lastRun = 0;
      decisionNow = -1;
    },
    shouldRun(now: number) {
      if (targetFps <= 0) {
        return true;
      }
      if (now === decisionNow) {
        return decisionAllow;
      }
      decisionNow = now;
      const interval = 1000 / targetFps;
      if (lastRun === 0) {
        lastRun = now;
        decisionAllow = true;
        return true;
      }
      const elapsed = now - lastRun;
      if (elapsed < interval) {
        decisionAllow = false;
        return false;
      }
      // Advance by whole intervals so brief stalls do not permanently lag.
      const steps = Math.floor(elapsed / interval);
      lastRun += steps * interval;
      // Cap catch-up so a long pause does not schedule a burst of "owed" work.
      if (now - lastRun > interval) {
        lastRun = now;
      }
      decisionAllow = true;
      return true;
    },
  };
}

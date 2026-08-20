export interface MouseTrailEngine {
  setMouse(x: number, y: number): void;
  leaveScreen(): void;
  start(): void;
  stop(): void;
  destroy(): void;
  resize(): void;
}

export type MouseTrailEffect = "ribbon" | "meteor" | "graffiti" | "dots";

export interface MouseTrailPref {
  enabled: boolean;
  effect: MouseTrailEffect;
}

export const DEFAULT_MOUSE_TRAIL_PREF: MouseTrailPref = {
  enabled: false,
  effect: "ribbon",
};

export function normalizeMouseTrailEffect(raw: string): MouseTrailEffect {
  if (raw === "meteor") {
    return "meteor";
  }
  if (raw === "graffiti") {
    return "graffiti";
  }
  if (raw === "dots") {
    return "dots";
  }
  return "ribbon";
}

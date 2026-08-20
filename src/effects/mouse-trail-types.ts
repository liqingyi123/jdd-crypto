export interface MouseTrailEngine {
  setMouse(x: number, y: number): void;
  leaveScreen(): void;
  start(): void;
  stop(): void;
  destroy(): void;
  resize(): void;
}

export type MouseTrailEffect = "ribbon" | "meteor";

export interface MouseTrailPref {
  enabled: boolean;
  effect: MouseTrailEffect;
}

export const DEFAULT_MOUSE_TRAIL_PREF: MouseTrailPref = {
  enabled: false,
  effect: "ribbon",
};

export function normalizeMouseTrailEffect(raw: string): MouseTrailEffect {
  return raw === "meteor" ? "meteor" : "ribbon";
}

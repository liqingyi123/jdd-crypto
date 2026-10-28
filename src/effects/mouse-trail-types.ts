export interface MouseTrailEngine {
  setMouse(x: number, y: number): void;
  leaveScreen(): void;
  start(): void;
  stop(): void;
  destroy(): void;
  resize(): void;
  setColor?(color: string): void;
  /** 0 = uncapped (follow display refresh). */
  setTargetFps?(fps: number): void;
}

export type MouseTrailEffect =
  | "ribbon"
  | "meteor"
  | "graffiti"
  | "dots"
  | "heart"
  | "ripple"
  | "bubbles"
  | "bloom"
  | "fall";

export type ColorableTrailEffect = "meteor" | "dots" | "heart" | "bloom" | "fall";

export interface MouseTrailColors {
  meteor: string;
  dots: string;
  heart: string;
  /** Stored for preference compat; ripple uses fixed mid-gray highlight/shadow (not user-tintable). */
  ripple: string;
  bloom: string;
  fall: string;
}

export type MouseTrailPerformanceMode = "eco" | "balanced" | "perf";

export interface MouseTrailPref {
  enabled: boolean;
  effect: MouseTrailEffect;
  performanceMode: MouseTrailPerformanceMode;
  colors: MouseTrailColors;
}

export const DEFAULT_MOUSE_TRAIL_COLORS: MouseTrailColors = {
  meteor: "#F8EC85",
  dots: "#00D1CE",
  heart: "#FF2EC8",
  ripple: "#2A2A2E",
  bloom: "#7AD7FF",
  fall: "#FFB86B",
};

export const DEFAULT_MOUSE_TRAIL_PREF: MouseTrailPref = {
  enabled: false,
  effect: "ribbon",
  performanceMode: "balanced",
  colors: { ...DEFAULT_MOUSE_TRAIL_COLORS },
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
  if (raw === "heart") {
    return "heart";
  }
  if (raw === "ripple") {
    return "ripple";
  }
  if (raw === "bubbles") {
    return "bubbles";
  }
  if (raw === "bloom") {
    return "bloom";
  }
  if (raw === "fall") {
    return "fall";
  }
  return "ribbon";
}

export function normalizeMouseTrailPerformanceMode(
  raw: string | undefined,
): MouseTrailPerformanceMode {
  if (raw === "eco" || raw === "perf" || raw === "balanced") {
    return raw;
  }
  return "balanced";
}

export function isColorableTrailEffect(
  effect: MouseTrailEffect,
): effect is ColorableTrailEffect {
  return (
    effect === "meteor" ||
    effect === "dots" ||
    effect === "heart" ||
    effect === "bloom" ||
    effect === "fall"
  );
}

/** Normalize to #RRGGBB; invalid input returns fallback. */
export function normalizeTrailColor(raw: string, fallback: string): string {
  const cleaned = raw.trim().replace(/^#/, "").toLowerCase();
  if (/^[0-9a-f]{6}$/.test(cleaned)) {
    return `#${cleaned}`;
  }
  const fb = fallback.trim().replace(/^#/, "").toLowerCase();
  if (/^[0-9a-f]{6}$/.test(fb)) {
    return `#${fb}`;
  }
  return "#ffffff";
}

export function normalizeMouseTrailColors(
  raw: Partial<MouseTrailColors> | undefined,
): MouseTrailColors {
  const d = DEFAULT_MOUSE_TRAIL_COLORS;
  return {
    meteor: normalizeTrailColor(raw?.meteor ?? d.meteor, d.meteor),
    dots: normalizeTrailColor(raw?.dots ?? d.dots, d.dots),
    heart: normalizeTrailColor(raw?.heart ?? d.heart, d.heart),
    ripple: normalizeTrailColor(raw?.ripple ?? d.ripple, d.ripple),
    bloom: normalizeTrailColor(raw?.bloom ?? d.bloom, d.bloom),
    fall: normalizeTrailColor(raw?.fall ?? d.fall, d.fall),
  };
}

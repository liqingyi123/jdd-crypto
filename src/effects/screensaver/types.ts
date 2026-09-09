export type ScreensaverBackground = "parallax" | "corona" | "snow";
export type ScreensaverClock = "lcd3d";

export type ScreensaverPref = {
  background: ScreensaverBackground;
  clock: ScreensaverClock;
};

export const DEFAULT_SCREENSAVER_PREF: ScreensaverPref = {
  background: "parallax",
  clock: "lcd3d",
};

export function normalizeScreensaverBackground(raw: unknown): ScreensaverBackground {
  if (raw === "corona" || raw === "snow" || raw === "parallax") {
    return raw;
  }
  return "parallax";
}

export function normalizeScreensaverClock(_raw: unknown): ScreensaverClock {
  return "lcd3d";
}

export function normalizeScreensaverPref(raw: unknown): ScreensaverPref {
  if (!raw || typeof raw !== "object") {
    return { ...DEFAULT_SCREENSAVER_PREF };
  }
  const obj = raw as Record<string, unknown>;
  // Legacy: { effect: "parallax" }
  if (typeof obj.effect === "string" && obj.background == null) {
    return {
      background: normalizeScreensaverBackground(obj.effect),
      clock: "lcd3d",
    };
  }
  return {
    background: normalizeScreensaverBackground(obj.background),
    clock: normalizeScreensaverClock(obj.clock),
  };
}

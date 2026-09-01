import { computed, shallowRef } from "vue";
import {
  DEFAULT_MOUSE_TRAIL_COLORS,
  isColorableTrailEffect,
  type MouseTrailEffect,
} from "@/effects/mouse-trail-types";

const effect = shallowRef<MouseTrailEffect>("dots");
const color = shallowRef(DEFAULT_MOUSE_TRAIL_COLORS.meteor);

function defaultColorFor(next: MouseTrailEffect): string {
  if (next === "meteor") {
    return DEFAULT_MOUSE_TRAIL_COLORS.meteor;
  }
  if (next === "dots") {
    return DEFAULT_MOUSE_TRAIL_COLORS.dots;
  }
  if (next === "heart") {
    return DEFAULT_MOUSE_TRAIL_COLORS.heart;
  }
  return DEFAULT_MOUSE_TRAIL_COLORS.meteor;
}

export function useGlobalTrail() {
  const colorable = computed(() => isColorableTrailEffect(effect.value));

  function setEffect(next: MouseTrailEffect) {
    effect.value = next;
    if (isColorableTrailEffect(next)) {
      color.value = defaultColorFor(next);
    }
  }

  function setColor(next: string) {
    color.value = next;
  }

  return {
    effect,
    color,
    colorable,
    setEffect,
    setColor,
  };
}

/** Parse only JSON object/array strings; leave scalars and invalid JSON as-is. */
function parseJsonObjectOrArray(text: string): object | undefined {
  const trimmed = text.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
    return undefined;
  }
  try {
    const parsed: unknown = JSON.parse(trimmed);
    if (parsed !== null && typeof parsed === "object") {
      return parsed;
    }
  } catch {
    // keep original string
  }
  return undefined;
}

/**
 * Expand stringified JSON object/array values one level.
 * Nested string fields inside a freshly parsed value stay strings so display
 * keeps at most one escape level (e.g. body expands, lastIds stays `\"...\"`).
 */
export function expandEmbeddedJson(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => {
      if (typeof item === "string") {
        return parseJsonObjectOrArray(item) ?? item;
      }
      return expandEmbeddedJson(item);
    });
  }
  if (value !== null && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
      if (typeof child === "string") {
        out[key] = parseJsonObjectOrArray(child) ?? child;
      } else {
        out[key] = expandEmbeddedJson(child);
      }
    }
    return out;
  }
  return value;
}

/** Pretty-print JSON and expand one level of embedded JSON strings. */
export function beautifyJsonDisplay(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed) {
    return raw;
  }
  if (
    !(trimmed.startsWith("{") || trimmed.startsWith("[") || trimmed.startsWith('"'))
  ) {
    return raw;
  }
  try {
    const parsed: unknown = JSON.parse(trimmed);
    return JSON.stringify(expandEmbeddedJson(parsed), null, 2);
  } catch {
    return raw;
  }
}

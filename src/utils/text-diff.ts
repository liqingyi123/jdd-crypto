export type DiffKind = "equal" | "add" | "change";

export interface DiffSegment {
  text: string;
  kind: DiffKind;
}

/** Split into tokens: whitespace-separated words keep trailing spaces with next token loosely. */
function tokenize(text: string): string[] {
  if (!text) {
    return [];
  }
  return text.split(/(\s+)/).filter((part) => part.length > 0);
}

/**
 * Build segments for the right-hand text, highlighting additions/changes vs left.
 * Uses a simple LCS over tokens (word/whitespace).
 */
export function diffHighlightRight(left: string, right: string): DiffSegment[] {
  const a = tokenize(left);
  const b = tokenize(right);
  if (a.length === 0 && b.length === 0) {
    return [];
  }
  if (a.length === 0) {
    return b.map((text) => ({ text, kind: "add" as const }));
  }
  if (b.length === 0) {
    return [];
  }

  const n = a.length;
  const m = b.length;
  const dp: number[][] = Array.from({ length: n + 1 }, () => Array(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      if (a[i] === b[j]) {
        dp[i][j] = dp[i + 1][j + 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i + 1][j], dp[i][j + 1]);
      }
    }
  }

  const matchedB = new Set<number>();
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      matchedB.add(j);
      i += 1;
      j += 1;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      i += 1;
    } else {
      j += 1;
    }
  }

  const segments: DiffSegment[] = [];
  for (let k = 0; k < m; k += 1) {
    const kind: DiffKind = matchedB.has(k) ? "equal" : "add";
    const last = segments[segments.length - 1];
    if (last && last.kind === kind) {
      last.text += b[k];
    } else {
      segments.push({ text: b[k], kind });
    }
  }
  return segments;
}

/** Pretty-print JSON when possible. */
export function beautifyText(raw: string): string {
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
    return JSON.stringify(JSON.parse(trimmed), null, 2);
  } catch {
    return raw;
  }
}

/**
 * Punch transparent corners on the app master icon.
 * Flood-fills from the four canvas corners through near-navy background
 * pixels so rounded-rect icons no longer show opaque black/blue corners.
 *
 * Usage: node ./scripts/fix-icon-alpha.mjs
 */
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const iconPath = path.join(root, "src-tauri", "icons", "icon.png");
const appIconPath = path.join(root, "public", "app-icon.png");

const MAX_COLOR_DIST = 60;
const MAX_LUMA = 90;

function loadPngjs() {
  try {
    return createRequire(import.meta.url)("pngjs");
  } catch {
    // fall through — load from a temp npm pack (no package.json change)
  }

  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "jdd-pngjs-"));
  execSync("npm pack pngjs", { cwd: tmp, stdio: "pipe" });
  const tgz = fs.readdirSync(tmp).find((name) => name.endsWith(".tgz"));
  if (!tgz) {
    throw new Error("npm pack pngjs produced no tarball");
  }
  execSync(`tar -xzf "${tgz}"`, { cwd: tmp, stdio: "pipe" });
  return createRequire(path.join(tmp, "package", "package.json"))(
    path.join(tmp, "package"),
  );
}

function luma(r, g, b) {
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function isBackground(r, g, b, a, seed) {
  if (a === 0) {
    return false;
  }
  // Keep neon border / bright content (key, cyan, magenta trails).
  if (luma(r, g, b) > MAX_LUMA) {
    return false;
  }
  if (Math.max(r, g, b) > 110) {
    return false;
  }
  const dr = r - seed[0];
  const dg = g - seed[1];
  const db = b - seed[2];
  return Math.sqrt(dr * dr + dg * dg + db * db) <= MAX_COLOR_DIST;
}

function punchCorners(png) {
  const { width: w, height: h, data } = png;
  const visited = new Uint8Array(w * h);
  const queue = [];

  const push = (x, y) => {
    if (x < 0 || y < 0 || x >= w || y >= h) {
      return;
    }
    const idx = y * w + x;
    if (visited[idx]) {
      return;
    }
    visited[idx] = 1;
    queue.push(idx);
  };

  const seeds = [
    [0, 0],
    [w - 1, 0],
    [0, h - 1],
    [w - 1, h - 1],
  ];

  for (const [sx, sy] of seeds) {
    const i = (sy * w + sx) * 4;
    const seed = [data[i], data[i + 1], data[i + 2]];
    // Reset visit mask per seed so each corner flood stays local to that region.
    visited.fill(0);
    queue.length = 0;
    push(sx, sy);

    while (queue.length > 0) {
      const idx = queue.pop();
      const x = idx % w;
      const y = (idx / w) | 0;
      const p = idx * 4;
      const r = data[p];
      const g = data[p + 1];
      const b = data[p + 2];
      const a = data[p + 3];
      if (!isBackground(r, g, b, a, seed)) {
        continue;
      }
      data[p + 3] = 0;
      push(x + 1, y);
      push(x - 1, y);
      push(x, y + 1);
      push(x, y - 1);
    }
  }
}

function cornerStats(png) {
  const { width: w, height: h, data } = png;
  const at = (x, y) => {
    const i = (y * w + x) * 4;
    return [data[i], data[i + 1], data[i + 2], data[i + 3]];
  };
  return {
    tl: at(0, 0),
    tr: at(w - 1, 0),
    bl: at(0, h - 1),
    br: at(w - 1, h - 1),
  };
}

function main() {
  const { PNG } = loadPngjs();
  const input = fs.readFileSync(iconPath);
  const png = PNG.sync.read(input);
  console.log("before", cornerStats(png));
  punchCorners(png);
  console.log("after", cornerStats(png));

  const out = PNG.sync.write(png);
  fs.writeFileSync(iconPath, out);
  fs.writeFileSync(appIconPath, out);
  console.log(`wrote ${path.relative(root, iconPath)}`);
  console.log(`wrote ${path.relative(root, appIconPath)}`);
}

main();

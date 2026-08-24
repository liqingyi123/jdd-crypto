import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);

function run(child) {
  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 1);
  });
}

if (process.platform === "win32") {
  const bat = path.join(root, "scripts", "tauri.cmd");
  run(
    spawn(process.env.ComSpec || "cmd.exe", ["/d", "/s", "/c", bat, ...args], {
      cwd: root,
      stdio: "inherit",
      windowsHide: true,
      env: process.env,
    }),
  );
} else {
  const tauriBin = path.join(root, "node_modules", ".bin", "tauri");
  run(
    spawn(tauriBin, args, {
      cwd: root,
      stdio: "inherit",
      env: process.env,
    }),
  );
}

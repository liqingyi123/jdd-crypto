import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const bat = path.join(root, "scripts", "tauri.cmd");
const args = process.argv.slice(2);

const child = spawn(process.env.ComSpec || "cmd.exe", ["/d", "/s", "/c", bat, ...args], {
  cwd: root,
  stdio: "inherit",
  windowsHide: true,
  env: process.env,
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});

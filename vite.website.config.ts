import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const projectRoot = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig(({ command }) => ({
  root: fileURLToPath(new URL("./website", import.meta.url)),
  // 开发用绝对 base，避免入口脚本解析失败白屏；构建用相对路径便于子目录部署
  base: command === "build" ? "./" : "/",
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  publicDir: fileURLToPath(new URL("./public", import.meta.url)),
  server: {
    port: 5173,
    strictPort: true,
    open: "/",
    fs: {
      allow: [projectRoot],
    },
  },
  build: {
    outDir: fileURLToPath(new URL("./dist-website", import.meta.url)),
    emptyOutDir: true,
    target: "chrome105",
  },
}));

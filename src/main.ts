import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/theme.css";

async function resolveWindowLabel(): Promise<string> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    return getCurrentWindow().label;
  } catch {
    const params = new URLSearchParams(window.location.search);
    return params.get("window") || "main";
  }
}

async function bootstrap() {
  const windowLabel = await resolveWindowLabel();

  if (windowLabel === "badge") {
    document.documentElement.classList.add("badge-window");
    document.body.style.background = "transparent";
  } else {
    document.documentElement.classList.remove("badge-window");
    // Element Plus dark CSS vars only needed for main/feature windows.
    await import("element-plus/theme-chalk/dark/css-vars.css");
  }

  const app = createApp(App, { windowLabel });
  app.use(createPinia());
  app.mount("#app");
}

void bootstrap();

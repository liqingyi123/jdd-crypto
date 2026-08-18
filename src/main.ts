import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { useAppStore } from "./stores/app";
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
  const app = createApp(App, { windowLabel });
  const pinia = createPinia();
  app.use(pinia);
  useAppStore(pinia).setWindowLabel(windowLabel);

  if (windowLabel === "badge") {
    document.documentElement.classList.add("badge-window");
    document.body.style.background = "transparent";
  } else {
    document.documentElement.classList.remove("badge-window");
  }

  app.mount("#app");
}

void bootstrap();

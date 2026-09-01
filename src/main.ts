import "./styles/shell.css";
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";

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
  const isMouseTrail = windowLabel.startsWith("mouse-trail");
  const isTransparentChrome =
    windowLabel === "badge" ||
    windowLabel === "clipboard-prompt" ||
    windowLabel === "crypto-bubble" ||
    windowLabel === "compare-tip" ||
    windowLabel === "compare-bubble";

  if (isMouseTrail) {
    // Keep trail overlays free of theme chrome (transparent only).
    document.documentElement.classList.add("badge-window");
    document.body.style.background = "transparent";
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().setIgnoreCursorEvents(true);
    } catch {
      // browser preview
    }
  } else if (windowLabel === "compare-tip") {
    await import("./styles/theme.css");
    document.documentElement.classList.add("badge-window");
    document.body.style.background = "transparent";
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().setIgnoreCursorEvents(true);
    } catch {
      // browser preview
    }
  } else if (isTransparentChrome) {
    // Badge / clipboard prompt need theme tokens; stay transparent via badge-window.
    await import("./styles/theme.css");
    document.documentElement.classList.add("badge-window");
    document.body.style.background = "transparent";
  } else {
    await import("./styles/theme.css");
    document.documentElement.classList.remove("badge-window");
    await import("element-plus/theme-chalk/dark/css-vars.css");
  }

  const app = createApp(App, { windowLabel });
  app.use(createPinia());
  app.mount("#app");
}

void bootstrap();

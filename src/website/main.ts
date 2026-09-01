import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import { useWebsiteTheme } from "./composables/use-website-theme";
import "./styles/website.css";

const { initTheme } = useWebsiteTheme();
initTheme();

createApp(App).use(router).mount("#app");

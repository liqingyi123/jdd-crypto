import { createRouter, createWebHashHistory } from "vue-router";
import HomeView from "./views/home.vue";
import FeaturesView from "./views/features.vue";
import AboutView from "./views/about.vue";
import ChangelogView from "./views/changelog.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: HomeView },
    { path: "/features", name: "features", component: FeaturesView },
    { path: "/about", name: "about", component: AboutView },
    { path: "/changelog", name: "changelog", component: ChangelogView },
  ],
  scrollBehavior() {
    return { top: 0 };
  },
});

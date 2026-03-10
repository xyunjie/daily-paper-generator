import { createRouter, createWebHashHistory } from "vue-router";
import Home from "../pages/Home.vue";
import Settings from "../pages/Settings.vue";
import Records from "../pages/Records.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: Home },
    { path: "/records", name: "records", component: Records },
    { path: "/settings", name: "settings", component: Settings },
  ],
});

export default router;

import { createRouter, createWebHashHistory } from "vue-router";
import Home from "../pages/Home.vue";
import Settings from "../pages/Settings.vue";
import Records from "../pages/Records.vue";
import Logs from "../pages/Logs.vue";
import WeeklyReport from "../pages/WeeklyReport.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: Home },
    { path: "/weekly", name: "weekly", component: WeeklyReport },
    { path: "/records", name: "records", component: Records },
    { path: "/settings", name: "settings", component: Settings },
    { path: "/logs", name: "logs", component: Logs },
  ],
});

export default router;

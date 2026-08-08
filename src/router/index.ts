import { createRouter, createWebHistory } from "vue-router";
import MainLayout from "@/layouts/MainLayout.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      component: MainLayout,
      children: [
        { path: "", redirect: "/root" },
        {
          path: "tools",
          name: "tools",
          component: () => import("@/views/Tools.vue"),
        },
        {
          path: "root",
          name: "root",
          component: () => import("@/views/Root.vue"),
        },
      ],
    },
  ],
});

export default router;

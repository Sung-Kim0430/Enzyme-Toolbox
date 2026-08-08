<script setup lang="ts">
import { computed } from "vue";
import { useRouter, useRoute } from "vue-router";
import TitleBar from "@/components/TitleBar.vue";

const router = useRouter();
const route = useRoute();

const tabs = [
  { label: "工具", value: "/tools" },
  { label: "Root", value: "/root" },
];

const activeTab = computed(() => {
  if (route.path.startsWith("/tools")) return "/tools";
  return "/root";
});

function handleTabChange(value: string) {
  router.push(value);
}
</script>

<template>
  <n-layout class="main-layout">
    <!-- 自定义标题栏 -->
    <TitleBar />

    <!-- 顶部工具栏 -->
    <n-layout-header class="main-header">
      <div class="header-content">
        <n-tabs
          :value="activeTab"
          type="segment"
          size="small"
          class="header-tabs"
          @update:value="handleTabChange"
        >
          <n-tab-pane
            v-for="tab in tabs"
            :key="tab.value"
            :name="tab.value"
            :tab="tab.label"
          />
        </n-tabs>
      </div>
    </n-layout-header>

    <!-- 中间内容区 -->
    <n-layout-content class="main-content">
      <router-view />
    </n-layout-content>

    <!-- 底部 Footer -->
    <n-layout-footer bordered class="main-footer">
      <div class="footer-content">
        <span>Copyright &copy; 2026 酶游明（Enzymeym）. T5M Unlock Tool 保留所有权利。</span>
      </div>
    </n-layout-footer>
  </n-layout>
</template>

<style scoped>
.main-layout {
  height: 100vh;
  overflow: hidden;
}

.main-layout :deep(.n-layout-scroll-container) {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.main-header {
  flex-shrink: 0;
  padding: 0 16px;
  height: 42px;
  display: flex;
  align-items: center;
}

.header-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.header-tabs {
  --n-tab-font-size: 12px;
  --n-tab-padding: 2px 10px;
  --n-tab-gap: 2px;
  flex: none;
  width: 160px;
}

.main-content {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 24px 40px;
}

.main-footer {
  flex-shrink: 0;
  margin-top: auto;
  padding: 0 24px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-color);
}

.footer-content {
  font-size: 13px;
  color: var(--n-text-color-3);
}
</style>

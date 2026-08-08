<script setup lang="ts">
import { ref, onMounted } from "vue";
import { MinimizeLine, FullscreenLine, CloseLine } from "@mingcute/vue";

interface TauriWindow {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  isMaximized(): Promise<boolean>;
}

const isMaximized = ref(false);
const isTauri = ref(false);
let appWindow: TauriWindow | null = null;

async function initTauri() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    appWindow = getCurrentWindow();
    isTauri.value = true;
    isMaximized.value = await appWindow.isMaximized();
  } catch {
    isTauri.value = false;
  }
}

onMounted(() => {
  initTauri();
});

async function onMinimize() {
  await appWindow?.minimize();
}

async function onToggleMaximize() {
  if (!appWindow) return;
  await appWindow.toggleMaximize();
  isMaximized.value = await appWindow.isMaximized();
}

async function onClose() {
  await appWindow?.close();
}
</script>

<template>
  <div class="titlebar" :data-tauri-drag-region="isTauri ? '' : undefined">
    <div class="titlebar-title">
      <span>Enzyme Toolbox</span>
      <n-tag size="tiny" type="error">Dev</n-tag>
    </div>
    <div v-if="isTauri" class="titlebar-controls">
      <n-button
        quaternary
        size="tiny"
        class="ctrl-btn"
        @click="onMinimize"
      >
        <template #icon>
          <MinimizeLine :size="18" />
        </template>
      </n-button>
      <n-button
        quaternary
        size="tiny"
        class="ctrl-btn"
        @click="onToggleMaximize"
      >
        <template #icon>
          <FullscreenLine :size="18" />
        </template>
      </n-button>
      <n-button
        quaternary
        size="tiny"
        class="ctrl-btn ctrl-btn--close"
        @click="onClose"
      >
        <template #icon>
          <CloseLine :size="18" />
        </template>
      </n-button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 38px;
  padding: 0 6px 0 12px;
  background: var(--n-color);
  border-bottom: 1px solid var(--n-border-color);
  user-select: none;
  flex-shrink: 0;
}

.titlebar-title {
  font-size: 13px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--n-text-color-2);
  padding-left: 4px;
}

.titlebar-controls {
  display: flex;
  align-items: center;
  gap: 2px;
}

.ctrl-btn {
  width: 34px;
  height: 28px;
  padding: 0;
  border-radius: 4px;
}

.ctrl-btn--close:hover {
  background: #e81123 !important;
  color: #fff !important;
}
</style>

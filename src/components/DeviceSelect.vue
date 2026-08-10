<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { CellphoneLine } from "@mingcute/vue";

interface AdbDevice {
  serial: string;
  state: string;
}

const props = withDefaults(defineProps<{ disabled?: boolean }>(), {
  disabled: false,
});

const emit = defineEmits<{
  (e: "select", serial: string | null): void;
}>();

const devices = ref<AdbDevice[]>([]);
const selectedSerial = ref<string | null>(null);
const loading = ref(false);
const errorMsg = ref("");

const selectedDevice = computed(() => {
  return devices.value.find((d) => d.serial === selectedSerial.value) ?? null;
});

const selectOptions = computed(() =>
  devices.value.map((d) => ({
    label: `${d.serial} (${d.state})`,
    value: d.serial,
  }))
);

async function refreshDevices() {
  loading.value = true;
  errorMsg.value = "";
  try {
    devices.value = await invoke<AdbDevice[]>("get_adb_devices", {
      adbPath: null,
    });
    if (devices.value.length === 0) {
      errorMsg.value = "未检测到 ADB 设备";
    }
    if (!devices.value.find((d) => d.serial === selectedSerial.value)) {
      selectedSerial.value = devices.value.length > 0 ? devices.value[0].serial : null;
      emit("select", selectedSerial.value);
    }
  } catch (e) {
    errorMsg.value = `${e}`;
  } finally {
    loading.value = false;
  }
}

function handleSelect(serial: string | null) {
  selectedSerial.value = serial;
  emit("select", serial);
}

onMounted(() => {
  refreshDevices();
});

defineExpose({ selectedSerial, selectedDevice, refreshDevices });
</script>

<template>
  <div class="device-select">
    <n-select
      :value="selectedSerial"
      :options="selectOptions"
      :loading="loading"
      :disabled="props.disabled"
      :placeholder="errorMsg || '选择 ADB 设备'"
      :consistent-menu-width="false"
      clearable
      class="device-dropdown"
      @update:value="handleSelect"
    >
      <template #empty>
        <div class="empty-tip">
          <span v-if="loading">正在扫描设备...</span>
          <span v-else>{{ errorMsg || '未检测到设备' }}</span>
        </div>
      </template>
    </n-select>
    <n-button
      quaternary
      :loading="loading"
      :disabled="props.disabled"
      @click="refreshDevices"
    >
      <template #icon>
        <CellphoneLine :size="18" />
      </template>
    </n-button>
  </div>
</template>

<style scoped>
.device-select {
  display: flex;
  align-items: center;
  gap: 8px;
}

.device-dropdown {
  width: 280px;
}

.empty-tip {
  padding: 8px 12px;
  color: var(--n-text-color-3);
  font-size: 13px;
}
</style>

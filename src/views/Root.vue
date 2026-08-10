<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DeviceSelect from "@/components/DeviceSelect.vue";

interface AdbCheckInfo {
  path: string;
  version: string;
}

interface EscalationStep {
  name: string;
  ok: boolean;
  output: string;
}

interface EscalationResult {
  success: boolean;
  steps: EscalationStep[];
}

const STEPS = ["准备", "检查 ADB", "选择设备", "开始提权"];

const stepIndex = ref(0);

const logs = ref<string[]>([]);
const logText = computed(() => logs.value.join("\n"));

const checkingAdb = ref(false);
const adbChecked = ref(false);
const adbInfo = ref<AdbCheckInfo | null>(null);
const adbError = ref("");

const selectedSerial = ref<string | null>(null);

function onDeviceSelect(serial: string | null) {
  selectedSerial.value = serial;
}

const escalating = ref(false);
const escalateResult = ref<EscalationResult | null>(null);
const escalateError = ref("");

const exporting = ref(false);
const exportResult = ref("");

function pushLog(line: string) {
  const time = new Date().toLocaleTimeString("zh-CN", { hour12: false });
  logs.value.push(`[${time}] ${line}`);
}

/// 指定步骤是否满足进入条件
function canAccess(i: number): boolean {
  if (i <= 1) return true;
  if (i === 2) return adbChecked.value;
  return adbChecked.value && !!selectedSerial.value;
}

function goStep(i: number) {
  if (i > stepIndex.value && !canAccess(i)) {
    if (i === 2) pushLog("请先完成第 1 步：检查 ADB 安装");
    if (i === 3) pushLog("请先完成第 1、2 步：检查 ADB 并选择设备");
    return;
  }
  stepIndex.value = i;
}

function nextStep() {
  goStep(stepIndex.value + 1);
}

function prevStep() {
  stepIndex.value = Math.max(0, stepIndex.value - 1);
}

function stepStatus(i: number) {
  if (i === 0) return "finish";
  if (i === 1) {
    if (checkingAdb.value) return "process";
    if (adbChecked.value) return "finish";
    if (adbError.value) return "error";
  }
  if (i === 2) return selectedSerial.value ? "finish" : "wait";
  if (i === 3) {
    if (escalating.value) return "process";
    if (escalateResult.value) return escalateResult.value.success ? "finish" : "error";
  }
  return "wait";
}

async function checkAdb() {
  checkingAdb.value = true;
  adbError.value = "";
  pushLog("===== 开始检查 ADB 安装 =====");
  try {
    const info = await invoke<AdbCheckInfo>("check_adb_installation", {
      adbPath: null,
    });
    adbInfo.value = info;
    adbChecked.value = true;
    pushLog("ADB 已就绪");
    pushLog(`路径: ${info.path}`);
    pushLog(`版本: ${info.version}`);
  } catch (e) {
    adbError.value = `${e}`;
    adbChecked.value = false;
    pushLog(`ADB 检查失败: ${e}`);
  } finally {
    checkingAdb.value = false;
  }
}

watch(selectedSerial, (serial) => {
  if (serial) {
    escalateResult.value = null;
    escalateError.value = "";
    exportResult.value = "";
    pushLog(`已选择设备: ${serial}`);
  }
});

async function startEscalation() {
  const serial = selectedSerial.value;
  if (!serial) return;
  escalating.value = true;
  escalateResult.value = null;
  escalateError.value = "";
  pushLog(`===== 开始对设备 ${serial} 提权 ====`);
  try {
    const result = await invoke<EscalationResult>("escalate_privileges", {
      adbPath: null,
      serial,
    });
    escalateResult.value = result;
    for (const step of result.steps) {
      pushLog(`[${step.ok ? "成功" : "失败"}] ${step.name}`);
      if (step.output) pushLog(`  ${step.output}`);
    }
    pushLog(result.success ? ">>> 提权成功 <<<" : ">>> 提权失败 <<<");
  } catch (e) {
    escalateError.value = `${e}`;
    pushLog(`提权异常: ${e}`);
  } finally {
    escalating.value = false;
  }
}

async function exportLogs() {
  exporting.value = true;
  exportResult.value = "";
  try {
    const path = await invoke<string>("export_logs", {
      content: logText.value,
      defaultName: `t5m_root_log_${Date.now()}.txt`,
    });
    exportResult.value = `已导出: ${path}`;
    pushLog(`日志已导出: ${path}`);
  } catch (e) {
    exportResult.value = `${e}`;
    pushLog(`${e}`);
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <div class="root-page">
    <h2>Root 提权</h2>

    <div class="main-split">
      <div class="left-col">
    <!-- 横向步骤条 -->
    <n-steps
      horizontal
      :current="stepIndex"
      :status="'process'"
      class="step-bar"
      @update:current="goStep"
    >
      <n-step
        v-for="(title, i) in STEPS"
        :key="i"
        :title="title"
        :status="stepStatus(i)"
      />
    </n-steps>

    <!-- 准备页 -->
    <n-card v-if="stepIndex === 0" class="step-card" :bordered="false">
      <template #header>准备</template>
      <div class="prep-body">
        <p>本工具用于在 T5m 305 设备上通过 LD_PRELOAD 注入临时获取 root 权限，请按以下步骤操作：</p>
        <ol class="prep-list">
          <li><b>检查 ADB 安装</b>：未安装时工具会自动下载 adb 套件。</li>
          <li><b>选择设备</b>：选择已连接并开启 USB 调试的设备。</li>
          <li><b>开始提权</b>：执行注入并验证提权结果。</li>
        </ol>
        <p class="prep-tip">提示：注入命令最多等待 30 秒；成功后将以 <code>su -c 'id'</code> 输出 <code>uid=0(root)</code> 验证。</p>
      </div>
    </n-card>

    <!-- 步骤 1：检查 ADB -->
    <n-card v-else-if="stepIndex === 1" class="step-card" :bordered="false">
      <template #header>检查 ADB 安装</template>
      <div class="step-body">
        <n-button type="primary" :loading="checkingAdb" @click="checkAdb">
          检查 ADB
        </n-button>
        <n-alert
          v-if="adbChecked"
          type="success"
          :bordered="false"
          class="step-tip"
        >
          ADB 已就绪<br />
          {{ adbInfo?.path }}<br />
          {{ adbInfo?.version }}
        </n-alert>
        <n-alert v-else-if="adbError" type="error" :bordered="false" class="step-tip">
          {{ adbError }}
        </n-alert>
        <n-alert v-else type="info" :bordered="false" class="step-tip">
          点击"检查 ADB"开始。若未安装，将自动下载 adb 套件。
        </n-alert>
      </div>
    </n-card>

    <!-- 步骤 2：选择设备 -->
    <n-card v-else-if="stepIndex === 2" class="step-card" :bordered="false">
      <template #header>选择设备</template>
      <div class="step-body">
        <DeviceSelect :disabled="!adbChecked" @select="onDeviceSelect" />
        <n-alert v-if="!adbChecked" type="info" :bordered="false" class="step-tip">
          请先完成第 1 步：检查 ADB 安装。
        </n-alert>
      </div>
    </n-card>

    <!-- 步骤 3：开始提权 -->
    <n-card v-else class="step-card" :bordered="false">
      <template #header>开始提权</template>
      <div class="step-body">
        <n-button
          type="primary"
          :loading="escalating"
          :disabled="!adbChecked || !selectedSerial"
          @click="startEscalation"
        >
          开始提权
        </n-button>
        <n-alert
          v-if="escalateResult?.success"
          type="success"
          :bordered="false"
          class="step-tip"
        >
          提权成功，设备已获得 root 权限
        </n-alert>
        <n-alert
          v-else-if="escalateError || escalateResult"
          type="error"
          :bordered="false"
          class="step-tip"
        >
          {{ escalateError || "提权失败，请查看日志" }}
        </n-alert>
        <n-alert v-else type="info" :bordered="false" class="step-tip">
          选择设备后点击"开始提权"。
        </n-alert>
      </div>
    </n-card>

    <!-- 上一步 / 下一步 -->
    <div class="step-nav">
      <n-button :disabled="stepIndex === 0" @click="prevStep">
        上一步
      </n-button>
      <n-button
        v-if="stepIndex < STEPS.length - 1"
        type="primary"
        @click="nextStep"
      >
        下一步
      </n-button>
    </div>
      </div>

      <!-- 日志区域（右栏） -->
      <div class="right-col">
        <div class="log-section">
          <div class="log-header">
            <span class="log-title">执行日志</span>
            <n-button
              size="small"
              :loading="exporting"
              :disabled="logs.length === 0"
              @click="exportLogs"
            >
              导出日志
            </n-button>
          </div>
          <n-log :log="logText" :rows="14" />
          <div v-if="exportResult" class="export-result">{{ exportResult }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.root-page {
  width: 100%;
  margin: 0 auto;
}

.main-split {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}

.left-col {
  flex: 1;
  min-width: 0;
}

.right-col {
  width: 340px;
  flex-shrink: 0;
}

.step-bar {
  margin-bottom: 16px;
}

.step-card {
  background: var(--n-color, #fff);
}

.prep-body {
  color: var(--n-text-color-2);
  font-size: 14px;
}

.prep-list {
  padding-left: 20px;
  line-height: 2;
}

.prep-tip {
  color: var(--n-text-color-3);
  font-size: 13px;
}

.step-body {
  padding: 4px 0;
}

.step-tip {
  margin-top: 10px;
}

.step-nav {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}

.log-section {
  margin-top: 16px;
}

.log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.log-title {
  font-weight: 600;
}

.export-result {
  margin-top: 8px;
  font-size: 13px;
  color: var(--n-text-color-3);
  word-break: break-all;
}
</style>

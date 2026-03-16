<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { message } from "ant-design-vue";

const logContent = ref("");
const logPath = ref("");
const loading = ref(false);
let autoRefreshTimer: ReturnType<typeof setInterval> | null = null;
const autoRefresh = ref(false);

async function loadLog() {
  loading.value = true;
  try {
    logContent.value = await invoke<string>("read_log_file");
  } catch (e) {
    message.error(`读取日志失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

async function loadLogPath() {
  try {
    logPath.value = await invoke<string>("get_log_path");
  } catch (e) {
    // ignore
  }
}

function clearDisplay() {
  logContent.value = "";
}

function toggleAutoRefresh(checked: boolean) {
  if (checked) {
    autoRefreshTimer = setInterval(loadLog, 3000);
  } else {
    if (autoRefreshTimer) {
      clearInterval(autoRefreshTimer);
      autoRefreshTimer = null;
    }
  }
}

onMounted(async () => {
  await loadLogPath();
  await loadLog();
});

onUnmounted(() => {
  if (autoRefreshTimer) clearInterval(autoRefreshTimer);
});
</script>

<template>
  <div class="logs-container">
    <div class="logs-toolbar">
      <div class="logs-title">运行日志</div>
      <div class="logs-actions">
        <span class="log-path" :title="logPath">{{ logPath }}</span>
        <a-switch
          v-model:checked="autoRefresh"
          checked-children="自动刷新"
          un-checked-children="自动刷新"
          @change="toggleAutoRefresh"
        />
        <a-button :loading="loading" @click="loadLog">刷新</a-button>
        <a-button @click="clearDisplay">清空显示</a-button>
      </div>
    </div>
    <div class="log-viewer">
      <pre v-if="logContent" class="log-content">{{ logContent }}</pre>
      <div v-else class="log-empty">暂无日志</div>
    </div>
  </div>
</template>

<style scoped>
.logs-container {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 64px - 48px);
  gap: 12px;
}

.logs-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.logs-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.logs-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.log-path {
  font-size: 12px;
  color: var(--text-muted);
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-viewer {
  flex: 1;
  overflow: auto;
  background: #1e1e1e;
  border-radius: 8px;
  padding: 12px 16px;
}

.log-content {
  margin: 0;
  font-family: "JetBrains Mono", "Fira Code", "Cascadia Code", monospace;
  font-size: 12px;
  line-height: 1.6;
  color: #d4d4d4;
  white-space: pre-wrap;
  word-break: break-all;
}

.log-empty {
  color: var(--text-secondary);
  text-align: center;
  padding-top: 40px;
}
</style>

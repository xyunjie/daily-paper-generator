<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { message } from "ant-design-vue";
import dayjs, { type Dayjs } from "dayjs";
import isoWeek from "dayjs/plugin/isoWeek";
import {
  initDb,
  listWorkItems,
  saveWeekSummary,
  getWeekSummary,
  listWeekSummaries,
  countWeekSummaries,
  type WorkItem,
  type WeekSummary,
} from "../db";
import { buildWeekDates } from "../utils/date";
import { invoke } from "@tauri-apps/api/core";

dayjs.extend(isoWeek);

// 当前选中周的周一
const selectedWeekStart = ref<Dayjs>(dayjs().isoWeekday(1).startOf("day"));
const thisWeekStart = dayjs().isoWeekday(1).startOf("day");

const weekLabel = computed(() => {
  const start = selectedWeekStart.value;
  const end = start.add(6, "day");
  const isThisWeek = start.isSame(thisWeekStart, "day");
  return isThisWeek
    ? `本周（${start.format("MM/DD")} - ${end.format("MM/DD")}）`
    : `${start.format("YYYY/MM/DD")} - ${end.format("MM/DD")}`;
});

function disabledWeek(date: Dayjs) {
  return date.isoWeekday(1).startOf("day").isAfter(thisWeekStart);
}

function onWeekChange(date: Dayjs | null) {
  if (!date) return;
  selectedWeekStart.value = date.isoWeekday(1).startOf("day");
  loadWeekData();
}

function prevWeek() {
  selectedWeekStart.value = selectedWeekStart.value.subtract(1, "week");
  loadWeekData();
}

function nextWeek() {
  const next = selectedWeekStart.value.add(1, "week");
  if (next.isAfter(thisWeekStart)) return;
  selectedWeekStart.value = next;
  loadWeekData();
}

const isCurrentWeek = computed(() => selectedWeekStart.value.isSame(thisWeekStart, "day"));

// 工作项数据
interface DayGroup {
  dateStr: string;
  weekday: string;
  items: WorkItem[];
}

const loading = ref(false);
const dayGroups = ref<DayGroup[]>([]);
const weekSummary = ref("");
const keyTasks = ref("");
const completionStatus = ref("");
const summaryLoading = ref(false);
const tasksLoading = ref(false);
const exportLoading = ref(false);

// 历史周报
const historyList = ref<WeekSummary[]>([]);
const historyTotal = ref(0);
const historyPage = ref(1);
const historyPageSize = 10;

onMounted(async () => {
  try {
    await initDb();
    await loadWeekData();
    await loadHistory();
  } catch (e) {
    message.error(`初始化失败: ${e}`);
  }
});

async function loadWeekData() {
  loading.value = true;
  try {
    const base = selectedWeekStart.value.toDate();
    const week = buildWeekDates(base);
    const start = week[0].dateStr;
    const end = week[6].dateStr;
    const rows = await listWorkItems(start, end);

    dayGroups.value = week.map((d) => ({
      dateStr: d.dateStr,
      weekday: d.weekday,
      items: rows.filter((r) => r.work_date === d.dateStr),
    }));

    const weekStartStr = selectedWeekStart.value.format("YYYY-MM-DD");
    const summaryRecord = await getWeekSummary(weekStartStr);
    weekSummary.value = summaryRecord?.summary || "";
    keyTasks.value = summaryRecord?.key_tasks || "";
    completionStatus.value = summaryRecord?.completion_status || "";
  } finally {
    loading.value = false;
  }
}

async function handleSummarize() {
  const allItems = dayGroups.value.flatMap((g) => g.items.map((i) => i.content)).filter(Boolean);
  if (allItems.length === 0) {
    message.warning("本周暂无工作内容");
    return;
  }
  summaryLoading.value = true;
  weekSummary.value = "";
  try {
    const summary = await invoke<string>("summarize_week", {
      itemsJson: JSON.stringify(allItems),
    });
    weekSummary.value = summary;
    const weekStartStr = selectedWeekStart.value.format("YYYY-MM-DD");
    await saveWeekSummary(weekStartStr, summary, keyTasks.value, completionStatus.value);
    await loadHistory();
    message.success("总结已生成并保存");
  } catch (e) {
    message.error(`总结失败: ${e}`);
  } finally {
    summaryLoading.value = false;
  }
}

async function handleGenerateTasks() {
  const allItems = dayGroups.value.flatMap((g) => g.items.map((i) => i.content)).filter(Boolean);
  if (allItems.length === 0) {
    message.warning("本周暂无工作内容");
    return;
  }
  tasksLoading.value = true;
  try {
    const [kt, cs] = await invoke<[string, string]>("generate_week_tasks", {
      itemsJson: JSON.stringify(allItems),
    });
    keyTasks.value = kt;
    completionStatus.value = cs;
    const weekStartStr = selectedWeekStart.value.format("YYYY-MM-DD");
    await saveWeekSummary(weekStartStr, weekSummary.value, kt, cs);
    await loadHistory();
    message.success("重点任务已生成并保存");
  } catch (e) {
    message.error(`生成失败: ${e}`);
  } finally {
    tasksLoading.value = false;
  }
}

async function handleExport() {
  exportLoading.value = true;
  try {
    const base = selectedWeekStart.value.toDate();
    const week = buildWeekDates(base);
    const startDate = week[0].dateStr;
    const endDate = week[6].dateStr;
    const dayItems = dayGroups.value.map((g) => ({
      date: g.dateStr,
      contents: g.items.map((i) => i.content),
    }));
    await invoke("export_week_report", {
      startDate,
      endDate,
      itemsJson: JSON.stringify(dayItems),
      summary: weekSummary.value,
      keyTasks: keyTasks.value,
      completionStatus: completionStatus.value,
      employee: "",
    });
    message.success("周报已保存");
  } catch (e) {
    const msg = String(e);
    if (msg.includes("已取消")) return;
    message.error(`导出失败: ${e}`);
  } finally {
    exportLoading.value = false;
  }
}

async function loadHistory() {
  const offset = (historyPage.value - 1) * historyPageSize;
  historyTotal.value = await countWeekSummaries();
  historyList.value = await listWeekSummaries(historyPageSize, offset);
}

function onHistoryPageChange(page: number) {
  historyPage.value = page;
  loadHistory();
}

function formatWeekRange(weekStart: string) {
  const start = dayjs(weekStart);
  const end = start.add(6, "day");
  return `${start.format("YYYY/MM/DD")} - ${end.format("MM/DD")}`;
}

const weeklyColumns = [
  { title: "日期", dataIndex: "dateStr", width: 120 },
  { title: "星期", dataIndex: "weekday", width: 80 },
  { title: "工作内容", dataIndex: "items", key: "items" },
];

const historyColumns = [
  { title: "周", dataIndex: "week_start", key: "week_start", width: 200 },
  { title: "总结内容", dataIndex: "summary", key: "summary", ellipsis: true },
  { title: "更新时间", dataIndex: "updated_at", key: "updated_at", width: 180 },
];
</script>

<template>
  <div class="weekly-container">
    <!-- 周导航 -->
    <div class="toolbar">
      <div class="week-nav">
        <a-button size="small" @click="prevWeek">&lt;</a-button>
        <a-date-picker
          picker="week"
          :value="selectedWeekStart"
          :disabled-date="disabledWeek"
          format="YYYY第WW周"
          :allow-clear="false"
          size="small"
          @change="onWeekChange"
        />
        <a-button size="small" :disabled="isCurrentWeek" @click="nextWeek">&gt;</a-button>
        <span class="week-label">{{ weekLabel }}</span>
      </div>
      <div class="actions">
        <a-button :loading="summaryLoading" @click="handleSummarize">AI 总结</a-button>
        <a-button type="primary" :loading="exportLoading" @click="handleExport">导出周报</a-button>
      </div>
    </div>

    <!-- 本周工作内容表格 -->
    <a-spin :spinning="loading">
      <a-table
        :columns="weeklyColumns"
        :data-source="dayGroups"
        :pagination="false"
        row-key="dateStr"
        size="small"
        bordered
      >
        <template #bodyCell="{ column, record }">
          <template v-if="column.key === 'items'">
            <div v-if="record.items.length === 0" class="empty-text">暂无记录</div>
            <ul v-else class="work-list">
              <li v-for="item in record.items" :key="item.id">{{ item.content }}</li>
            </ul>
          </template>
        </template>
      </a-table>
    </a-spin>

    <!-- 周总结 -->
    <div class="summary-section">
      <div class="summary-header">
        <span class="summary-title">周工作总结</span>
      </div>
      <a-spin v-if="summaryLoading" class="summary-spin" />
      <a-textarea
        v-else
        v-model:value="weekSummary"
        :rows="4"
        placeholder="点击「AI 总结」自动生成，或手动输入周总结内容"
        @blur="async () => {
          if (weekSummary) {
            const weekStartStr = selectedWeekStart.format('YYYY-MM-DD');
            await saveWeekSummary(weekStartStr, weekSummary, keyTasks, completionStatus);
          }
        }"
      />
    </div>

    <!-- 重点任务 + 完成情况 -->
    <div class="tasks-section">
      <div class="tasks-header">
        <span class="section-title">本周重点任务</span>
        <a-button size="small" :loading="tasksLoading" @click="handleGenerateTasks">AI 生成</a-button>
      </div>
      <a-spin v-if="tasksLoading" class="summary-spin" />
      <a-textarea
        v-else
        v-model:value="keyTasks"
        :rows="5"
        placeholder="点击「AI 生成」自动提炼，或手动输入本周重点任务"
        @blur="async () => {
          const weekStartStr = selectedWeekStart.format('YYYY-MM-DD');
          await saveWeekSummary(weekStartStr, weekSummary, keyTasks, completionStatus);
        }"
      />
    </div>

    <div class="tasks-section">
      <div class="tasks-header">
        <span class="section-title">任务完成情况</span>
      </div>
      <a-spin v-if="tasksLoading" class="summary-spin" />
      <a-textarea
        v-else
        v-model:value="completionStatus"
        :rows="5"
        placeholder="点击重点任务区域的「AI 生成」同时生成，或手动输入完成情况"
        @blur="async () => {
          const weekStartStr = selectedWeekStart.format('YYYY-MM-DD');
          await saveWeekSummary(weekStartStr, weekSummary, keyTasks, completionStatus);
        }"
      />
    </div>

    <!-- 历史周报 -->
    <div class="history-section">
      <div class="history-header">
        <span class="history-title">历史周报</span>
      </div>
      <a-table
        :columns="historyColumns"
        :data-source="historyList"
        :pagination="{
          current: historyPage,
          pageSize: historyPageSize,
          total: historyTotal,
          showSizeChanger: false,
          onChange: onHistoryPageChange,
        }"
        row-key="id"
        size="small"
      >
        <template #bodyCell="{ column, record }">
          <template v-if="column.key === 'week_start'">
            {{ formatWeekRange(record.week_start) }}
          </template>
          <template v-if="column.key === 'updated_at'">
            {{ record.updated_at?.slice(0, 19).replace('T', ' ') }}
          </template>
        </template>
      </a-table>
    </div>
  </div>
</template>

<style>
.weekly-container {
  padding: 8px 12px;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.week-nav {
  display: flex;
  align-items: center;
  gap: 8px;
}

.week-label {
  font-size: 14px;
  color: var(--text-secondary);
}

.actions {
  display: flex;
  gap: 8px;
}

.empty-text {
  color: var(--text-muted);
  font-size: 13px;
}

.work-list {
  margin: 0;
  padding-left: 16px;
}

.work-list li {
  line-height: 1.6;
}

.summary-section {
  margin-top: 20px;
  padding: 16px 20px;
  background: var(--bg-section);
  border: 1px solid var(--bg-section-border);
  border-radius: 8px;
}

.summary-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.summary-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.summary-spin {
  display: block;
  text-align: center;
  padding: 16px 0;
}

.history-section {
  margin-top: 24px;
}

.history-header {
  margin-bottom: 12px;
}

.history-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.tasks-section {
  margin-top: 16px;
  padding: 16px 20px;
  background: var(--bg-section);
  border: 1px solid var(--bg-section-border);
  border-radius: 8px;
}

.tasks-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.section-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}
</style>

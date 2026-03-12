<script setup lang="ts">
import { onMounted, ref } from "vue";
import { message } from "ant-design-vue";
import dayjs, { type Dayjs } from "dayjs";
import { initDb, listWorkItems, replaceWorkItems, type WorkItem } from "../db";
import { buildWeekDates } from "../utils/date";
import { invoke } from "@tauri-apps/api/core";

interface FetchedItem {
  content: string;
  source: "jira" | "gitlab";
}

interface DailyCard {
  date: Date;
  dateStr: string;
  weekday: string;
  items: WorkItem[];
  fetchLoading: boolean;
  polishLoading: boolean;
}

const cards = ref<DailyCard[]>([]);
const loading = ref(false);
const exportLoading = ref(false);

// 编辑弹窗
const editModalOpen = ref(false);
const editDate = ref<Dayjs>(dayjs());
const editContents = ref<string[]>([""]);
const editSaving = ref(false);

onMounted(async () => {
  try {
    await initDb();
    await loadWeek();
  } catch (e) {
    message.error(`初始化数据库失败: ${e}`);
  }
});

async function loadWeek() {
  loading.value = true;
  try {
    const week = buildWeekDates(new Date());
    const start = week[0].dateStr;
    const end = week[6].dateStr;
    const rows = await listWorkItems(start, end);
    cards.value = week.map((d) => ({
      ...d,
      items: rows.filter((r) => r.work_date === d.dateStr),
      fetchLoading: false,
      polishLoading: false,
    }));
  } finally {
    loading.value = false;
  }
}

async function handleAutoFetch(card: DailyCard) {
  card.fetchLoading = true;
  try {
    const result = await invoke<{ items: FetchedItem[] }>("fetch_daily_items", {
      date: card.dateStr,
    });
    const items = result?.items || [];
    await replaceWorkItems(card.dateStr, items.map((i) => ({ content: i.content, source: i.source })));
    await loadWeek();
    message.success(`已获取 ${items.length} 条记录`);
  } catch (e) {
    message.error(`自动获取失败: ${e}`);
  } finally {
    card.fetchLoading = false;
  }
}

async function handlePolish(card: DailyCard) {
  if (card.items.length === 0) {
    message.warning("请先自动获取数据");
    return;
  }
  card.polishLoading = true;
  try {
    const rawItems = card.items.map((i) => ({ content: i.content, source: i.source || "manual" }));
    const polished = await invoke<string[]>("polish_daily_items", {
      date: card.dateStr,
      itemsJson: JSON.stringify(rawItems),
    });
    await replaceWorkItems(card.dateStr, polished.map((c) => ({ content: c, source: "manual" as const })));
    await loadWeek();
    message.success("AI润色完成");
  } catch (e) {
    message.error(`AI润色失败: ${e}`);
  } finally {
    card.polishLoading = false;
  }
}

function openEditModal(card: DailyCard) {
  editDate.value = dayjs(card.dateStr);
  editContents.value = card.items.length ? card.items.map((i) => i.content) : [""];
  editModalOpen.value = true;
}

function addEditRow() {
  editContents.value.push("");
}

function removeEditRow(index: number) {
  if (editContents.value.length === 1) {
    editContents.value[0] = "";
    return;
  }
  editContents.value.splice(index, 1);
}

async function handleEditSave() {
  const dateStr = editDate.value.format("YYYY-MM-DD");
  const rows = editContents.value.map((v) => v.trim()).filter(Boolean);
  if (rows.length === 0) {
    message.warning("请至少填写一条工作内容");
    return;
  }
  editSaving.value = true;
  try {
    await replaceWorkItems(dateStr, rows.map((c) => ({ content: c, source: "manual" as const })));
    editModalOpen.value = false;
    await loadWeek();
    message.success("已保存");
  } finally {
    editSaving.value = false;
  }
}

async function exportWeek() {
  exportLoading.value = true;
  try {
    const week = buildWeekDates(new Date());
    const startDate = week[0].dateStr;
    const endDate = week[6].dateStr;
    const dayItems = cards.value.map((card) => ({
      date: card.dateStr,
      contents: card.items.map((item) => item.content),
    }));
    await invoke("export_week_report", {
      startDate,
      endDate,
      itemsJson: JSON.stringify(dayItems),
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

function sourceLabel(source: string) {
  if (source === "jira") return "Jira";
  if (source === "gitlab") return "GitLab";
  return null;
}

function sourceColor(source: string) {
  if (source === "jira") return "blue";
  if (source === "gitlab") return "orange";
  return "";
}
</script>

<template>
  <div class="home-container">
    <div class="toolbar">
      <div class="title">本周工作内容</div>
      <div class="actions">
        <a-button :loading="exportLoading" @click="exportWeek">导出本周工作内容</a-button>
      </div>
    </div>

    <div v-if="!loading" class="week-grid">
      <a-card
        v-for="card in cards"
        :key="card.dateStr"
        :title="`${card.dateStr}（${card.weekday}）`"
        class="day-card"
      >
        <div v-if="card.items.length === 0" class="empty-text">暂无记录</div>
        <ul v-else class="work-list">
          <li v-for="item in card.items" :key="item.id" class="work-item">
            <a-tag v-if="sourceLabel(item.source)" :color="sourceColor(item.source)" class="source-tag">
              {{ sourceLabel(item.source) }}
            </a-tag>
            <span>{{ item.content }}</span>
          </li>
        </ul>
        <template #actions>
          <a-button
            type="text"
            size="small"
            :loading="card.fetchLoading"
            :disabled="card.fetchLoading || card.polishLoading"
            @click="handleAutoFetch(card)"
          >
            自动获取
          </a-button>
          <a-button
            type="text"
            size="small"
            :loading="card.polishLoading"
            :disabled="card.fetchLoading || card.polishLoading"
            @click="handlePolish(card)"
          >
            AI润色
          </a-button>
          <a-button
            type="text"
            size="small"
            :disabled="card.fetchLoading || card.polishLoading"
            @click="openEditModal(card)"
          >
            编辑
          </a-button>
        </template>
      </a-card>
    </div>

    <a-spin v-else />

    <a-modal
      v-model:open="editModalOpen"
      title="编辑工作内容"
      ok-text="保存"
      :confirm-loading="editSaving"
      @ok="handleEditSave"
    >
      <a-form layout="vertical">
        <a-form-item label="日期">
          <a-date-picker v-model:value="editDate" format="YYYY-MM-DD" disabled />
        </a-form-item>
        <a-form-item label="工作内容">
          <div class="dynamic-list">
            <div v-for="(_row, index) in editContents" :key="index" class="dynamic-row">
              <a-input v-model:value="editContents[index]" placeholder="输入一条工作内容" />
              <a-button type="text" danger @click="removeEditRow(index)">删除</a-button>
            </div>
            <a-button type="dashed" block @click="addEditRow">+ 添加一行</a-button>
          </div>
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<style>
.home-container {
  padding: 8px 12px;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.title {
  font-size: 18px;
  font-weight: 600;
}

.actions {
  display: flex;
  gap: 8px;
}

.week-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
}

.day-card {
  min-height: 180px;
  display: flex;
  flex-direction: column;
}

.day-card :deep(.ant-card-body) {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.day-card :deep(.ant-card-actions) {
  margin-top: 0;
}

.dynamic-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.dynamic-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.work-list {
  padding-left: 0;
  margin: 0;
  list-style: none;
}

.work-item {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  margin-bottom: 6px;
  line-height: 1.5;
}

.source-tag {
  flex-shrink: 0;
  margin-top: 2px;
}

.empty-text {
  color: #999;
}
</style>


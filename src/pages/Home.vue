<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { message } from "ant-design-vue";
import dayjs, { type Dayjs } from "dayjs";
import { initDb, listWorkItems, replaceWorkItems, type WorkItem } from "../db";
import { buildWeekDates } from "../utils/date";
import { invoke } from "@tauri-apps/api/core";

interface DailyCard {
  date: Date;
  dateStr: string;
  weekday: string;
  items: WorkItem[];
}

const cards = ref<DailyCard[]>([]);
const loading = ref(false);
const addModalOpen = ref(false);
const addDate = ref<Dayjs>(dayjs());
const addContents = ref<string[]>([""]);
const autoFetchLoading = ref(false);
const exportLoading = ref(false);

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
    }));
  } finally {
    loading.value = false;
  }
}

function getItemsByDate(dateStr: string) {
  const card = cards.value.find((c) => c.dateStr === dateStr);
  return card ? card.items : [];
}

async function openAddModal() {
  const dateStr = addDate.value.format("YYYY-MM-DD");
  const existing = getItemsByDate(dateStr);
  addContents.value = existing.length ? existing.map((i) => i.content) : [""];
  addModalOpen.value = true;
}

watch(addDate, (newDate) => {
  if (!newDate) return;
  const dateStr = newDate.format("YYYY-MM-DD");
  const existing = getItemsByDate(dateStr);
  addContents.value = existing.length ? existing.map((i) => i.content) : [""];
});

function addContentRow() {
  addContents.value.push("");
}

function removeContentRow(index: number) {
  if (addContents.value.length === 1) {
    addContents.value[0] = "";
    return;
  }
  addContents.value.splice(index, 1);
}

async function handleAdd() {
  const dateStr = addDate.value.format("YYYY-MM-DD");
  const rows = addContents.value.map((v) => v.trim()).filter(Boolean);
  if (rows.length === 0) {
    message.warning("请至少填写一条工作内容");
    return;
  }
  await replaceWorkItems(dateStr, rows);
  addContents.value = [""];
  addModalOpen.value = false;
  await loadWeek();
  message.success("已保存");
}

async function handleAutoFetch() {
  const dateStr = addDate.value.format("YYYY-MM-DD");
  autoFetchLoading.value = true;
  try {
    const result = await invoke<{ tasks: string[]; commits: string[] }>("fetch_daily_items", {
      date: dateStr,
    });
    const items = [...(result?.tasks || []), ...(result?.commits || [])];
    // 清空之前的记录，写入新获取的内容
    const rows = items.filter((item) => item.trim());
    await replaceWorkItems(dateStr, rows);
    addContents.value = rows.length ? rows : [""];
    addModalOpen.value = false;
    await loadWeek();
    message.success("已自动获取并写入");
  } catch (e) {
    message.error(`自动获取失败: ${e}`);
  } finally {
    autoFetchLoading.value = false;
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
</script>

<template>
  <div class="home-container">
    <div class="toolbar">
      <div class="title">本周工作内容</div>
      <div class="actions">
        <a-button type="primary" @click="openAddModal">添加工作内容</a-button>
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
          <li v-for="item in card.items" :key="item.id">{{ item.content }}</li>
        </ul>
      </a-card>
    </div>

    <a-spin v-else />

    <a-modal
      v-model:open="addModalOpen"
      title="添加工作内容"
      ok-text="保存"
      @ok="handleAdd"
    >
      <a-form layout="vertical">
        <a-form-item label="日期">
          <a-date-picker v-model:value="addDate" format="YYYY-MM-DD" />
        </a-form-item>
        <a-form-item label="工作内容">
          <div class="dynamic-list">
            <div v-for="(_row, index) in addContents" :key="index" class="dynamic-row">
              <a-input
                v-model:value="addContents[index]"
                placeholder="输入一条工作内容"
              />
              <a-button type="text" danger @click="removeContentRow(index)">
                删除
              </a-button>
            </div>
            <a-button type="dashed" block @click="addContentRow">+ 添加一行</a-button>
          </div>
        </a-form-item>
      </a-form>
      <template #footer>
        <a-button @click="addModalOpen = false">取消</a-button>
        <a-button :loading="autoFetchLoading" :disabled="autoFetchLoading" @click="handleAutoFetch">自动获取</a-button>
        <a-button type="primary" :loading="autoFetchLoading" :disabled="autoFetchLoading" @click="handleAdd">保存</a-button>
      </template>
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
  padding-left: 18px;
  margin: 0;
}

.empty-text {
  color: #999;
}

</style>

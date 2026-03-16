<script setup lang="ts">
import { onMounted, ref } from "vue";
import { message } from "ant-design-vue";
import { type Dayjs } from "dayjs";
import {
  initDb,
  listWorkDaysPage,
  countWorkDays,
  type WorkDayRow,
} from "../db";

const loading = ref(false);
const data = ref<WorkDayRow[]>([]);
const total = ref(0);
const pageSize = ref(10);
const current = ref(1);

const keyword = ref("");
const dateRange = ref<[Dayjs, Dayjs] | null>(null);

const columns = [
  { title: "日期", dataIndex: "work_date", key: "work_date", width: 140 },
  { title: "工作内容", dataIndex: "contents", key: "contents" },
  { title: "条数", dataIndex: "item_count", key: "item_count", width: 90 },
];

onMounted(async () => {
  try {
    await initDb();
    await refresh();
  } catch (e) {
    message.error(`初始化数据库失败: ${e}`);
  }
});

async function refresh() {
  loading.value = true;
  try {
    const [start, end] = dateRange.value || [];
    const startDate = start ? start.format("YYYY-MM-DD") : undefined;
    const endDate = end ? end.format("YYYY-MM-DD") : undefined;
    total.value = await countWorkDays(keyword.value.trim(), startDate, endDate);
    const offset = (current.value - 1) * pageSize.value;
    data.value = await listWorkDaysPage(
      pageSize.value,
      offset,
      keyword.value.trim() || undefined,
      startDate,
      endDate
    );
  } finally {
    loading.value = false;
  }
}

async function onSearch() {
  current.value = 1;
  await refresh();
}

async function onReset() {
  keyword.value = "";
  dateRange.value = null;
  current.value = 1;
  await refresh();
}

async function onPageChange(page: number, size: number) {
  current.value = page;
  pageSize.value = size;
  await refresh();
}
</script>

<template>
  <a-card title="工作记录">
    <div class="filter-bar">
      <a-input
        v-model:value="keyword"
        allow-clear
        placeholder="搜索工作内容"
        class="filter-item"
      />
      <a-range-picker v-model:value="dateRange" class="filter-item" />
      <a-button type="primary" @click="onSearch">查询</a-button>
      <a-button @click="onReset">重置</a-button>
    </div>

    <a-table
      :columns="columns"
      :data-source="data"
      :loading="loading"
      row-key="work_date"
      :pagination="{
        current,
        pageSize,
        total,
        showSizeChanger: false,
        showTotal: (t: number) => `共 ${t} 天`,
        onChange: onPageChange,
      }"
      :row-class-name="() => 'work-row'"
    >
      <template #bodyCell="{ column, record }">
        <template v-if="column.key === 'contents'">
          <div class="contents-cell">
            <div v-for="(line, idx) in record.contents.split('\n')" :key="idx">
              {{ line }}
            </div>
          </div>
        </template>
      </template>
    </a-table>
  </a-card>
</template>

<style>
.filter-bar {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}

.filter-item {
  width: 240px;
}

.contents-cell {
  white-space: pre-line;
  color: var(--text-primary);
}

.work-row td {
  vertical-align: top;
}
</style>

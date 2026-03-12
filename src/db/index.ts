import Database from "@tauri-apps/plugin-sql";

const DB_PATH = "sqlite:daily.db";

let dbPromise: Promise<Database> | null = null;

export async function getDb() {
  if (!dbPromise) {
    dbPromise = Database.load(DB_PATH);
  }
  return dbPromise;
}

export async function initDb() {
  const db = await getDb();
  await db.execute(
    `CREATE TABLE IF NOT EXISTS work_items (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      work_date TEXT NOT NULL,
      content TEXT NOT NULL,
      source TEXT DEFAULT 'manual',
      created_at TEXT NOT NULL
    )`
  );
  // 为已存在的表添加source字段（如果不存在）
  try {
    await db.execute(`ALTER TABLE work_items ADD COLUMN source TEXT DEFAULT 'manual'`);
  } catch (e) {
    // 字段已存在，忽略错误
  }
  // 周总结表
  await db.execute(
    `CREATE TABLE IF NOT EXISTS week_summaries (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      week_start TEXT NOT NULL UNIQUE,
      summary TEXT NOT NULL,
      updated_at TEXT NOT NULL
    )`
  );
}

export interface WorkItem {
  id: number;
  work_date: string;
  content: string;
  source: 'jira' | 'gitlab' | 'manual';
  created_at: string;
}

export async function addWorkItem(workDate: string, content: string, source: 'jira' | 'gitlab' | 'manual' = 'manual') {
  const db = await getDb();
  const createdAt = new Date().toISOString();
  await db.execute(
    "INSERT INTO work_items (work_date, content, source, created_at) VALUES (?, ?, ?, ?)",
    [workDate, content, source, createdAt]
  );
}

export async function listWorkItems(startDate: string, endDate: string) {
  const db = await getDb();
  const rows = await db.select<WorkItem[]>(
    "SELECT * FROM work_items WHERE work_date >= ? AND work_date <= ? ORDER BY work_date ASC, id ASC",
    [startDate, endDate]
  );
  return rows;
}

export async function replaceWorkItems(workDate: string, items: Array<{ content: string; source?: 'jira' | 'gitlab' | 'manual' }>) {
  const db = await getDb();
  await db.execute("DELETE FROM work_items WHERE work_date = ?", [workDate]);
  const createdAt = new Date().toISOString();
  for (const item of items) {
    await db.execute(
      "INSERT INTO work_items (work_date, content, source, created_at) VALUES (?, ?, ?, ?)",
      [workDate, item.content, item.source ?? 'manual', createdAt]
    );
  }
}

export async function listWorkItemsPage(limit: number, offset: number) {
  const db = await getDb();
  const rows = await db.select<WorkItem[]>(
    "SELECT * FROM work_items ORDER BY work_date DESC, id DESC LIMIT ? OFFSET ?",
    [limit, offset]
  );
  return rows;
}

export async function countWorkItems() {
  const db = await getDb();
  const rows = await db.select<{ total: number }[]>(
    "SELECT COUNT(*) as total FROM work_items"
  );
  return Number(rows[0]?.total ?? 0);
}

export interface WorkDayRow {
  work_date: string;
  contents: string;
  item_count: number;
}

export async function listWorkDaysPage(
  limit: number,
  offset: number,
  keyword?: string,
  startDate?: string,
  endDate?: string
) {
  const db = await getDb();
  const filters: string[] = [];
  const params: Array<string | number> = [];

  if (startDate) {
    filters.push("work_date >= ?");
    params.push(startDate);
  }
  if (endDate) {
    filters.push("work_date <= ?");
    params.push(endDate);
  }
  if (keyword) {
    filters.push("content LIKE ?");
    params.push(`%${keyword}%`);
  }

  const where = filters.length ? `WHERE ${filters.join(" AND ")}` : "";

  const sql = `
    SELECT work_date,
           GROUP_CONCAT(content, '\n') as contents,
           COUNT(*) as item_count
    FROM work_items
    ${where}
    GROUP BY work_date
    ORDER BY work_date DESC
    LIMIT ? OFFSET ?
  `;

  const rows = await db.select<WorkDayRow[]>(sql, [...params, limit, offset]);
  return rows;
}

export async function countWorkDays(keyword?: string, startDate?: string, endDate?: string) {
  const db = await getDb();
  const filters: string[] = [];
  const params: Array<string | number> = [];

  if (startDate) {
    filters.push("work_date >= ?");
    params.push(startDate);
  }
  if (endDate) {
    filters.push("work_date <= ?");
    params.push(endDate);
  }
  if (keyword) {
    filters.push("content LIKE ?");
    params.push(`%${keyword}%`);
  }

  const where = filters.length ? `WHERE ${filters.join(" AND ")}` : "";

  const sql = `
    SELECT COUNT(DISTINCT work_date) as total
    FROM work_items
    ${where}
  `;

  const rows = await db.select<{ total: number }[]>(sql, params);
  return Number(rows[0]?.total ?? 0);
}

export async function deleteWorkItem(id: number) {
  const db = await getDb();
  await db.execute("DELETE FROM work_items WHERE id = ?", [id]);
}

export interface WeekSummary {
  id: number;
  week_start: string;
  summary: string;
  updated_at: string;
}

export async function saveWeekSummary(weekStart: string, summary: string) {
  const db = await getDb();
  const updatedAt = new Date().toISOString();
  await db.execute(
    "INSERT OR REPLACE INTO week_summaries (week_start, summary, updated_at) VALUES (?, ?, ?)",
    [weekStart, summary, updatedAt]
  );
}

export async function getWeekSummary(weekStart: string) {
  const db = await getDb();
  const rows = await db.select<WeekSummary[]>(
    "SELECT * FROM week_summaries WHERE week_start = ?",
    [weekStart]
  );
  return rows[0] || null;
}

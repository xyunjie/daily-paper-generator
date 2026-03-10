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
      created_at TEXT NOT NULL
    )`
  );
}

export interface WorkItem {
  id: number;
  work_date: string;
  content: string;
  created_at: string;
}

export async function addWorkItem(workDate: string, content: string) {
  const db = await getDb();
  const createdAt = new Date().toISOString();
  await db.execute(
    "INSERT INTO work_items (work_date, content, created_at) VALUES (?, ?, ?)",
    [workDate, content, createdAt]
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

export async function replaceWorkItems(workDate: string, contents: string[]) {
  const db = await getDb();
  await db.execute("DELETE FROM work_items WHERE work_date = ?", [workDate]);
  const createdAt = new Date().toISOString();
  for (const content of contents) {
    await db.execute(
      "INSERT INTO work_items (work_date, content, created_at) VALUES (?, ?, ?)",
      [workDate, content, createdAt]
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

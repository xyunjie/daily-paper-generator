import { addDays, format, getDay, startOfWeek } from "date-fns";
import type { Dayjs } from "dayjs";

export function getWeekStart(date: Date) {
  // 周一作为一周开始
  return startOfWeek(date, { weekStartsOn: 1 });
}

export function formatDate(date: Date) {
  // 使用本地时区格式化，避免 toISOString() 的 UTC 偏移导致日期错一天
  return format(date, "yyyy-MM-dd");
}

export function formatWeekday(date: Date) {
  // date-fns: 0=周日 ... 6=周六
  const day = getDay(date);
  const map = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
  return map[day] || "";
}

export function buildWeekDates(baseDate: Date) {
  const start = getWeekStart(baseDate);
  return Array.from({ length: 7 }).map((_, idx) => {
    const d = addDays(start, idx);
    return {
      date: d,
      dateStr: formatDate(d),
      weekday: formatWeekday(d),
    };
  });
}

export function dayjsToDateString(day: Dayjs | null) {
  if (!day) return "";
  return day.format("YYYY-MM-DD");
}

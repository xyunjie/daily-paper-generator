import { addDays, startOfWeek } from "date-fns";
import type { Dayjs } from "dayjs";

export function getWeekStart(date: Date) {
  return startOfWeek(date, { weekStartsOn: 1 });
}

export function formatDate(date: Date) {
  return date.toISOString().slice(0, 10);
}

export function buildWeekDates(baseDate: Date) {
  const start = getWeekStart(baseDate);
  return Array.from({ length: 7 }).map((_, idx) => {
    const d = addDays(start, idx);
    return {
      date: d,
      dateStr: formatDate(d),
    };
  });
}

export function dayjsToDateString(day: Dayjs | null) {
  if (!day) return "";
  return day.format("YYYY-MM-DD");
}

import type { HistoryRecord } from "../downloads/uiTypes";

export function historyDateLabel(timestamp: number) {
  const current = new Date();
  const date = new Date(timestamp);
  const currentDay = new Date(current.getFullYear(), current.getMonth(), current.getDate()).getTime();
  const recordDay = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const dayDifference = Math.round((currentDay - recordDay) / 86_400_000);
  if (dayDifference === 0) return "Hoje";
  if (dayDifference === 1) return "Ontem";
  return new Intl.DateTimeFormat("pt-BR", {
    day: "2-digit",
    month: "long",
    year: date.getFullYear() === current.getFullYear() ? undefined : "numeric",
  }).format(date);
}

export function groupHistory(items: HistoryRecord[]) {
  const groups = new Map<string, HistoryRecord[]>();
  items.forEach((item) => {
    const label = historyDateLabel(item.createdAt);
    groups.set(label, [...(groups.get(label) || []), item]);
  });
  return Array.from(groups, ([label, records]) => ({ label, records }));
}

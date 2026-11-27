// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

export interface DayGroup<T> {
  day: string;
  label: string;
  items: T[];
}

const localDay = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;

/** Newest first, split by the viewer's calendar day. */
export function groupByDay<T extends { timestamp: string }>(items: T[], now: Date = new Date()): DayGroup<T>[] {
  const today = localDay(now);
  const yesterday = localDay(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1));
  const groups = new Map<string, T[]>();
  const sorted = [...items].sort((a, b) => (Date.parse(b.timestamp) || 0) - (Date.parse(a.timestamp) || 0));
  for (const item of sorted) {
    const date = new Date(item.timestamp);
    const day = Number.isNaN(date.getTime()) ? 'unknown' : localDay(date);
    const group = groups.get(day);
    if (group) group.push(item);
    else groups.set(day, [item]);
  }
  return [...groups].map(([day, grouped]) => ({
    day,
    label: day === today ? 'Today' : day === yesterday ? 'Yesterday' : day === 'unknown' ? 'Unknown date' : day,
    items: grouped,
  }));
}

/** Local wall-clock time, or the raw value when it cannot be read. */
export function clockTime(timestamp: string): string {
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? timestamp : date.toLocaleTimeString([], { hour12: false });
}

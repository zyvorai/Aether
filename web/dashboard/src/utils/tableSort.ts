// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

export type SortDirection = 'asc' | 'desc';

export interface SortState {
  key: string;
  dir: SortDirection;
}

/** Click order on a column header: ascending, descending, then back to the page's own order. */
export function nextSort(current: SortState | null, key: string): SortState | null {
  if (!current || current.key !== key) return { key, dir: 'asc' };
  return current.dir === 'asc' ? { key, dir: 'desc' } : null;
}

/** Numbers by value, text naturally ("node-9" before "node-10"). Returns a new array. */
export function sortByValue<T>(items: T[], value: (item: T) => string | number, dir: SortDirection): T[] {
  const sign = dir === 'asc' ? 1 : -1;
  return [...items].sort((a, b) => {
    const av = value(a);
    const bv = value(b);
    if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * sign;
    return String(av).localeCompare(String(bv), undefined, { numeric: true, sensitivity: 'base' }) * sign;
  });
}

/** Where arrow keys, j/k, Home and End move the active row; null for other keys. */
export function moveIndex(key: string, index: number, length: number): number | null {
  if (length === 0) return null;
  if (key === 'ArrowDown' || key === 'j') return Math.min(length - 1, index + 1);
  if (key === 'ArrowUp' || key === 'k') return Math.max(0, index - 1);
  if (key === 'Home') return 0;
  if (key === 'End') return length - 1;
  return null;
}

// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState, type ReactNode } from 'react';
import ResponsiveTable from '../ResponsiveTable';
import { severityColorVar, severityRank, sortBySeverity } from '../../utils/severity';

export interface DataTableColumn<T> {
  key: string;
  header: string;
  width?: number;
  align?: 'left' | 'right';
  render: (item: T) => ReactNode;
  sortValue?: (item: T) => string | number;
}

interface DataTableProps<T> {
  items: T[];
  columns: DataTableColumn<T>[];
  getId: (item: T) => string;
  getStatus?: (item: T) => string | undefined | null;
  selectedId?: string | null;
  onSelect?: (item: T) => void;
  onOpen?: (item: T) => void;
  emptyTitle?: string;
  emptyBody?: string;
  /** Default is "worst first" via getStatus; pass false to keep item order as given. */
  sortBySeverityDefault?: boolean;
  testId?: string;
}

export default function DataTable<T>({
  items,
  columns,
  getId,
  getStatus,
  selectedId,
  onSelect,
  onOpen,
  emptyTitle = 'Nothing here',
  emptyBody = 'Clear filters, or widen the scope.',
  sortBySeverityDefault = true,
  testId,
}: DataTableProps<T>) {
  const [sort, setSort] = useState<{ key: string; dir: 'asc' | 'desc' } | null>(null);

  const rows = useMemo(() => {
    if (sort) {
      const col = columns.find((c) => c.key === sort.key);
      if (col?.sortValue) {
        const withKeys = [...items];
        withKeys.sort((a, b) => {
          const av = col.sortValue!(a);
          const bv = col.sortValue!(b);
          const cmp = av < bv ? -1 : av > bv ? 1 : 0;
          return sort.dir === 'asc' ? cmp : -cmp;
        });
        return withKeys;
      }
    }
    if (sortBySeverityDefault && getStatus) return sortBySeverity(items, getStatus);
    return items;
  }, [items, sort, columns, getStatus, sortBySeverityDefault]);

  const toggleSort = (key: string) => {
    setSort((s) => {
      if (!s || s.key !== key) return { key, dir: 'asc' };
      if (s.dir === 'asc') return { key, dir: 'desc' };
      return null;
    });
  };

  if (rows.length === 0) {
    return (
      <div className="px-6 py-16 text-center">
        <div className="text-[17px] font-semibold tracking-[-0.01em] text-foreground mb-2">{emptyTitle}</div>
        <div className="text-[13.5px] text-muted">{emptyBody}</div>
      </div>
    );
  }

  return (
    <ResponsiveTable testId={testId}>
      <div className="flex h-[30px] items-center border-b border-rule bg-surface pl-5 font-sans text-[11px] font-medium tracking-[0.02em] text-subtle">
        {columns.map((col) => (
          <button
            key={col.key}
            type="button"
            onClick={() => col.sortValue && toggleSort(col.key)}
            className={`shrink-0 truncate px-0 ${col.width ? '' : 'flex-1'} ${col.align === 'right' ? 'pr-[18px] text-right' : ''} ${
              col.sortValue ? 'cursor-pointer hover:text-muted' : 'cursor-default'
            }`}
            style={col.width ? { width: col.width } : undefined}
            disabled={!col.sortValue}
          >
            {col.header}
            {sort?.key === col.key ? (sort.dir === 'asc' ? ' ↑' : ' ↓') : ''}
          </button>
        ))}
      </div>

      <div>
        {rows.map((item) => {
          const id = getId(item);
          const status = getStatus?.(item);
          const isSelected = selectedId === id;
          const stripe = status ? severityColorVar(status) : null;
          const isBad = status ? severityRank(status) <= 7 : false;

          return (
            <div
              key={id}
              onClick={() => onSelect?.(item)}
              onDoubleClick={() => onOpen?.(item)}
              className="data-row relative flex cursor-pointer items-center border-b border-rule pl-5 font-mono text-[12.5px] text-muted transition-colors hover:bg-hover"
              style={{ background: isSelected ? 'var(--accent-tint)' : undefined }}
            >
              {(isBad || isSelected) && stripe ? (
                <span
                  className="absolute left-0 top-0 bottom-0 w-[3px]"
                  style={{ background: isBad ? stripe : 'var(--primary)' }}
                />
              ) : null}
              {columns.map((col) => (
                <div
                  key={col.key}
                  className={`shrink-0 truncate ${col.width ? '' : 'flex-1 min-w-0'} ${col.align === 'right' ? 'pr-[18px] text-right' : ''}`}
                  style={col.width ? { width: col.width } : undefined}
                >
                  {col.render(item)}
                </div>
              ))}
            </div>
          );
        })}
      </div>
    </ResponsiveTable>
  );
}

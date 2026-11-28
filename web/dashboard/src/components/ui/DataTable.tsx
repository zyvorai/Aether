// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { ArrowDown, ArrowUp, ChevronsUpDown } from 'lucide-react';
import { severityColorVar, severityRank, sortBySeverity } from '../../utils/severity';
import { entersTable, moveIndex, nextSort, sortByValue, type SortState } from '../../utils/tableSort';

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
  /** Accessible name of the table. */
  label?: string;
  /** Rows rendered before "Show more". */
  pageSize?: number;
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
  label = 'Results',
  pageSize = 100,
}: DataTableProps<T>) {
  const [sort, setSort] = useState<SortState | null>(null);
  const [limit, setLimit] = useState(pageSize);
  const [activeIndex, setActiveIndex] = useState(0);
  const [fits, setFits] = useState(false);
  const scrollerRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const interactive = Boolean(onSelect || onOpen);

  const rows = useMemo(() => {
    if (sort) {
      const col = columns.find((c) => c.key === sort.key);
      if (col?.sortValue) return sortByValue(items, col.sortValue, sort.dir);
    }
    if (sortBySeverityDefault && getStatus) return sortBySeverity(items, getStatus);
    return items;
  }, [items, sort, columns, getStatus, sortBySeverityDefault]);

  const visible = rows.slice(0, limit);
  const current = Math.min(activeIndex, Math.max(0, visible.length - 1));

  // The header can only stick to the page while the table needs no sideways scroll.
  useEffect(() => {
    const scroller = scrollerRef.current;
    if (!scroller || typeof ResizeObserver === 'undefined') return;
    const measure = () => {
      const content = scroller.firstElementChild as HTMLElement | null;
      setFits(Boolean(content) && content!.scrollWidth <= scroller.clientWidth + 1);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(scroller);
    return () => observer.disconnect();
  }, [rows.length]);

  // j or k pressed before any row has focus enters the table.
  useEffect(() => {
    if (!interactive) return;
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (!entersTable(event, event.target as HTMLElement | null, Boolean(document.querySelector('[aria-modal="true"]')))) return;
      const row = bodyRef.current?.children[current] as HTMLElement | undefined;
      if (!row) return;
      event.preventDefault();
      row.focus();
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [interactive, current]);

  if (rows.length === 0) {
    return (
      <div className="px-6 py-14 text-center" role="status">
        <div className="mb-2 text-[17px] font-semibold tracking-[-0.01em] text-foreground">{emptyTitle}</div>
        <div className="text-[13.5px] text-muted">{emptyBody}</div>
      </div>
    );
  }

  const onRowKeyDown = (event: KeyboardEvent<HTMLDivElement>, item: T, index: number) => {
    if (event.target !== event.currentTarget || event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const action = event.key === 'Enter' ? (onOpen ?? onSelect) : onSelect;
      action?.(item);
      return;
    }
    const next = moveIndex(event.key, index, visible.length);
    if (next === null) return;
    event.preventDefault();
    setActiveIndex(next);
    (bodyRef.current?.children[next] as HTMLElement | undefined)?.focus();
  };

  const cellClass = (col: DataTableColumn<T>) =>
    `data-cell ${col.width ? '' : 'flex-1 min-w-[12rem]'} ${col.align === 'right' ? 'data-cell-right' : ''}`;

  return (
    <div className="data-table rounded-2xl bg-surface-elevated shadow-card ring-1 ring-[var(--card-edge)]" data-testid={testId}>
      <div ref={scrollerRef} className="data-scroller" data-fits={fits ? 'true' : undefined}>
        <div className="min-w-fit" role="table" aria-label={label} aria-rowcount={rows.length + 1}>
          <div className="data-head" role="row">
            {columns.map((col) => {
              const style = col.width ? { width: col.width } : undefined;
              const direction = sort?.key === col.key ? sort.dir : null;
              if (!col.sortValue) {
                return (
                  <span key={col.key} role="columnheader" className={cellClass(col)} style={style}>
                    {col.header || <span className="sr-only">Select</span>}
                  </span>
                );
              }
              const Icon = direction === 'asc' ? ArrowUp : direction === 'desc' ? ArrowDown : ChevronsUpDown;
              return (
                <span
                  key={col.key}
                  role="columnheader"
                  aria-sort={direction === 'asc' ? 'ascending' : direction === 'desc' ? 'descending' : 'none'}
                  className={cellClass(col)}
                  style={style}
                >
                  <button type="button" className="data-sort" data-active={direction ? 'true' : undefined} onClick={() => setSort((s) => nextSort(s, col.key))}>
                    {col.header}
                    <Icon className="h-3 w-3" aria-hidden />
                  </button>
                </span>
              );
            })}
          </div>

          <div ref={bodyRef} role="rowgroup">
            {visible.map((item, index) => {
              const id = getId(item);
              const status = getStatus?.(item);
              const isSelected = selectedId === id;
              const stripe = status ? severityColorVar(status) : null;
              const isBad = status ? severityRank(status) <= 7 : false;

              return (
                <div
                  key={id}
                  role="row"
                  aria-selected={isSelected || undefined}
                  tabIndex={interactive ? (index === current ? 0 : -1) : undefined}
                  onFocus={interactive ? () => setActiveIndex(index) : undefined}
                  onKeyDown={interactive ? (event) => onRowKeyDown(event, item, index) : undefined}
                  onClick={() => {
                    // Selecting text in a row must not open it.
                    if (window.getSelection()?.toString()) return;
                    onSelect?.(item);
                  }}
                  onDoubleClick={() => onOpen?.(item)}
                  className={`data-row ${interactive ? 'data-row-interactive' : ''}`}
                >
                  {(isBad || isSelected) && stripe ? (
                    <span className="data-stripe" style={{ background: isBad ? stripe : 'var(--primary)' }} aria-hidden />
                  ) : null}
                  {columns.map((col) => (
                    <div
                      key={col.key}
                      role="cell"
                      data-label={col.header || undefined}
                      className={cellClass(col)}
                      style={col.width ? { width: col.width } : undefined}
                    >
                      {col.render(item)}
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        </div>
      </div>
      <div className="data-foot">
        <span aria-live="polite">
          {visible.length < rows.length ? `${visible.length} of ${rows.length} rows` : `${rows.length} ${rows.length === 1 ? 'row' : 'rows'}`}
        </span>
        {visible.length < rows.length ? (
          <button type="button" className="text-primary hover:underline" onClick={() => setLimit((n) => n + pageSize)}>
            Show more
          </button>
        ) : null}
        {interactive ? (
          <span className="data-hint">
            <kbd>j</kbd> <kbd>k</kbd> move · <kbd>Enter</kbd> open
          </span>
        ) : null}
      </div>
    </div>
  );
}

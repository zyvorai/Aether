// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, type KeyboardEvent } from 'react';

export interface SegmentedControlItem<T extends string> {
  key: T;
  label: string;
  count?: number;
  disabled?: boolean;
  tone?: string;
}

interface SegmentedControlProps<T extends string> {
  value: T;
  onChange: (key: T) => void;
  items: SegmentedControlItem<T>[];
  className?: string;
}

export default function SegmentedControl<T extends string>({
  value,
  onChange,
  items,
  className = '',
}: SegmentedControlProps<T>) {
  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
      const enabled = items.filter((i) => !i.disabled);
      if (enabled.length === 0) return;
      const idx = enabled.findIndex((i) => i.key === value);
      const delta = e.key === 'ArrowRight' ? 1 : -1;
      const next = enabled[(idx < 0 ? 0 : idx + delta + enabled.length) % enabled.length];
      e.preventDefault();
      onChange(next.key);
    },
    [items, value, onChange],
  );

  return (
    <div
      role="tablist"
      aria-label="Filter"
      onKeyDown={onKeyDown}
      className={`glass-fill inline-flex items-center gap-0.5 rounded-[var(--radius-md)] p-0.5 ${className}`}
    >
      {items.map((item) => {
        const isActive = item.key === value;
        return (
          <button
            key={item.key}
            type="button"
            role="tab"
            aria-selected={isActive}
            disabled={item.disabled}
            tabIndex={isActive ? 0 : -1}
            onClick={() => onChange(item.key)}
            className={[
              'flex items-center gap-1.5 whitespace-nowrap rounded-[7px] px-3 py-1.5 text-[12.5px] transition-colors',
              item.disabled ? 'cursor-default opacity-45 text-subtle' : 'cursor-pointer',
              isActive
                ? 'glass text-foreground font-medium'
                : item.disabled
                  ? ''
                  : `text-muted hover:text-foreground ${item.tone ?? ''}`,
            ].join(' ')}
          >
            {item.label}
            {item.count !== undefined ? (
              <span className={`font-mono text-[11.5px] ${isActive ? 'text-muted' : 'text-subtle'}`}>
                {item.count}
              </span>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}

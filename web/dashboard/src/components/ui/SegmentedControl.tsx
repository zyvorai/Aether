// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

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
  /** Accessible name of the group. */
  label?: string;
}

export default function SegmentedControl<T extends string>({
  value,
  onChange,
  items,
  className = '',
  label = 'Filter',
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
      aria-label={label}
      onKeyDown={onKeyDown}
      className={`segmented ${className}`}
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
            className={`segmented-item ${!isActive && !item.disabled ? (item.tone ?? '') : ''}`}
          >
            {item.label}
            {item.count !== undefined ? (
              <span className="segmented-count">{item.count}</span>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}

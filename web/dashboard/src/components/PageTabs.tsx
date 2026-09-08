// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useCallback, type ReactNode, type KeyboardEvent } from 'react';

export interface PageTab<T extends string> {
  id: T;
  label: string;
  icon?: ReactNode;
}

interface PageTabsProps<T extends string> {
  tabs: PageTab<T>[];
  active: T;
  onChange: (id: T) => void;
  className?: string;
}

export default function PageTabs<T extends string>({
  tabs,
  active,
  onChange,
  className = '',
}: PageTabsProps<T>) {
  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      const idx = tabs.findIndex((t) => t.id === active);
      if (idx < 0) return;
      if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
        e.preventDefault();
        const delta = e.key === 'ArrowRight' ? 1 : -1;
        const next = tabs[(idx + delta + tabs.length) % tabs.length];
        onChange(next.id);
      }
    },
    [active, onChange, tabs],
  );

  return (
    <div
      role="tablist"
      aria-label="Page sections"
      className={`mb-6 flex flex-wrap gap-2 ${className}`}
      onKeyDown={onKeyDown}
    >
      {tabs.map((tab) => {
        const isActive = active === tab.id;
        return (
          <button
            key={tab.id}
            type="button"
            role="tab"
            onClick={() => onChange(tab.id)}
            aria-selected={isActive}
            tabIndex={isActive ? 0 : -1}
            className={`glass-tab tab-chip flex items-center gap-2 ${isActive ? 'glass-tab-active tab-chip-active glass-tab-active' : ''}`}
          >
            {tab.icon}
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}

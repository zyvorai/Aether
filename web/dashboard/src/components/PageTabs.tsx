// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';

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
  return (
    <div className={`mb-6 flex flex-wrap gap-2 ${className}`}>
      {tabs.map((tab) => {
        const isActive = active === tab.id;
        return (
          <button
            key={tab.id}
            type="button"
            onClick={() => onChange(tab.id)}
            aria-selected={isActive}
            className={`tab-chip flex items-center gap-2 ${isActive ? 'tab-chip-active' : ''}`}
          >
            {tab.icon}
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}

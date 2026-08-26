// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, type ReactNode } from 'react';
import { X } from 'lucide-react';
import PageTabs, { type PageTab } from '../PageTabs';

export interface InspectorTab<T extends string> extends PageTab<T> {
  shortcut?: string;
}

interface InspectorProps<T extends string> {
  open: boolean;
  title: ReactNode;
  subtitle?: ReactNode;
  tabs: InspectorTab<T>[];
  activeTab: T;
  onTabChange: (id: T) => void;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}

export default function Inspector<T extends string>({
  open,
  title,
  subtitle,
  tabs,
  activeTab,
  onTabChange,
  onClose,
  children,
  footer,
}: InspectorProps<T>) {
  useEffect(() => {
    if (!open) return;
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const typing = target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName);
      if (typing) return;
      if (e.key === 'Escape') {
        onClose();
        return;
      }
      const tab = tabs.find((t) => t.shortcut === e.key);
      if (tab) onTabChange(tab.id);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open, tabs, onTabChange, onClose]);

  if (!open) return null;

  return (
    <aside
      className="animate-ac-slide flex w-full shrink-0 flex-col overflow-hidden border-l border-rule bg-raised sm:w-[45%] sm:min-w-[430px]"
      role="complementary"
    >
      <div className="shrink-0 px-5 pt-4">
        <div className="flex items-baseline gap-2.5">
          <div className="min-w-0 truncate font-mono text-[14px] text-ink">{title}</div>
          {subtitle ? <div className="shrink-0 text-[12.5px] font-medium">{subtitle}</div> : null}
          <span className="flex-1" />
          <button
            type="button"
            onClick={onClose}
            aria-label="Close inspector"
            className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-page text-ink-2 hover:text-ink"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>

        <PageTabs
          tabs={tabs.map((t) => ({
            id: t.id,
            label: t.shortcut ? `${t.label} (${t.shortcut.toUpperCase()})` : t.label,
            icon: t.icon,
          }))}
          active={activeTab}
          onChange={onTabChange}
          className="mt-4 mb-0 border-b border-rule pb-0"
        />
      </div>

      <div className="flex-1 overflow-auto px-5 py-4">{children}</div>

      {footer ? <div className="shrink-0 border-t border-rule px-5 py-2.5">{footer}</div> : null}
    </aside>
  );
}

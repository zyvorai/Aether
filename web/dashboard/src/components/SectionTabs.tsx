// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useRef } from 'react';
import { Link, useLocation } from 'react-router';
import type { AppView } from '../types/api';
import { getSectionTabs } from '../utils/navSections';
import { pathWithQuery } from '../utils/urlState';
import { cn } from '../lib/cn';

/** Tabs across a hub and its sub-pages. The workload in focus is carried over. */
export default function SectionTabs({ currentView }: { currentView: AppView }) {
  const { search } = useLocation();
  const activeRef = useRef<HTMLAnchorElement>(null);
  const listRef = useRef<HTMLElement>(null);
  const tabs = getSectionTabs(currentView);
  const workload = new URLSearchParams(search).get('workload') ?? undefined;

  useEffect(() => {
    const list = listRef.current;
    const active = activeRef.current;
    if (!list || !active) return;
    list.scrollLeft = Math.max(0, active.offsetLeft - list.clientWidth / 2 + active.clientWidth / 2);
  }, [currentView]);

  if (tabs.length === 0) return null;

  return (
    <nav ref={listRef} className="section-tabs" aria-label={`${tabs[0].label} sections`} data-testid="section-tabs">
      {tabs.map((tab, index) => {
        const active = tab.view === currentView;
        return (
          <Link
            key={tab.view}
            ref={active ? activeRef : undefined}
            to={pathWithQuery(tab.path, { workload })}
            aria-current={active ? 'page' : undefined}
            className={cn('section-tab', active && 'section-tab-active')}
          >
            {index === 0 ? 'Summary' : tab.label}
          </Link>
        );
      })}
    </nav>
  );
}

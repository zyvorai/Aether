// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { ChevronRight, Home } from 'lucide-react';
import { useNavigate } from 'react-router';
import type { AppView } from '../types/api';
import { VIEW_LABELS, getViewMeta, type NavGroup } from '../utils/dashboardNav';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';

interface BreadcrumbProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  workloadName?: string;
}

const GROUP_LABELS: Record<NavGroup, string> = {
  primary: 'Overview',
  intelligence: 'Intelligence',
  operations: 'Operations',
  resources: 'Resources',
};

// Explicit parent overrides for hub sub-pages (takes precedence over group).
const PARENT_CRUMBS: Partial<Record<AppView, { view: AppView; label: string }>> = {
  'ai-providers': { view: 'settings', label: 'Settings' },
};

export default function Breadcrumb({ currentView, onNavigate, workloadName }: BreadcrumbProps) {
  const navigate = useNavigate();
  if (currentView === 'overview') return null;

  const label = VIEW_LABELS[currentView];
  const workload = workloadName?.trim();
  const explicitParent = PARENT_CRUMBS[currentView];
  const group = (() => {
    try {
      return getViewMeta(currentView).group;
    } catch {
      return undefined;
    }
  })();
  const groupLabel = !explicitParent && group && group !== 'primary' ? GROUP_LABELS[group] : null;

  return (
    <nav className="dash-breadcrumb mb-3" aria-label="Breadcrumb">
      <div className="flex min-w-0 flex-wrap items-center gap-0.5 px-1 py-1">
        <button
          type="button"
          onClick={() => onNavigate('overview')}
          className="inline-flex items-center gap-1.5 rounded-lg px-2 py-1 text-[13px] text-ink-2 transition hover:bg-white/[0.04] hover:text-ink focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40"
        >
          <Home className="h-3.5 w-3.5 shrink-0 opacity-70" aria-hidden />
          <span className="hidden sm:inline">Home</span>
        </button>
        {explicitParent ? (
          <>
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-ink-3" aria-hidden />
            <button
              type="button"
              data-testid="breadcrumb-parent"
              onClick={() => onNavigate(explicitParent.view)}
              className="truncate rounded-lg px-2 py-1 text-[13px] text-ink-2 transition hover:bg-white/[0.04] hover:text-ink"
            >
              {explicitParent.label}
            </button>
          </>
        ) : null}
        {groupLabel ? (
          <>
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-ink-3" aria-hidden />
            <span data-testid="breadcrumb-group" className="truncate px-2 py-1 text-[13px] text-ink-3">
              {groupLabel}
            </span>
          </>
        ) : null}
        <ChevronRight className="h-3.5 w-3.5 shrink-0 text-ink-3" aria-hidden />
        <span className="truncate px-2 py-1 text-[13px] font-medium text-ink">{label}</span>
        {workload ? (
          <>
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-ink-3" aria-hidden />
            <button
              type="button"
              data-testid="breadcrumb-workload"
              onClick={() => navigate(pathWithQuery(viewToPath(currentView), { workload }))}
              className="max-w-[12rem] truncate rounded-lg px-2 py-1 text-[13px] font-mono text-brand transition hover:bg-white/[0.04] hover:underline"
            >
              {workload}
            </button>
          </>
        ) : null}
      </div>
    </nav>
  );
}

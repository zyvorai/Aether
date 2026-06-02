// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { ChevronRight, Home } from 'lucide-react';
import { useNavigate } from 'react-router';
import type { AppView } from '../types/api';
import { VIEW_LABELS } from '../utils/dashboardNav';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';

interface BreadcrumbProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  workloadName?: string;
}

const PARENT_CRUMBS: Partial<Record<AppView, { view: AppView; label: string }>> = {
  'ai-providers': { view: 'settings', label: 'Settings' },
};

export default function Breadcrumb({ currentView, onNavigate, workloadName }: BreadcrumbProps) {
  const navigate = useNavigate();
  if (currentView === 'overview') return null;

  const label = VIEW_LABELS[currentView];
  const workload = workloadName?.trim();
  const parent = PARENT_CRUMBS[currentView];

  return (
    <nav className="dash-breadcrumb mb-6" aria-label="Breadcrumb">
      <div className="flex min-w-0 flex-wrap items-center gap-1 px-1 py-1">
        <button
          type="button"
          onClick={() => onNavigate('overview')}
          className="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-sm text-slate-400 transition hover:bg-white/[0.04] hover:text-slate-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40"
        >
          <Home className="h-3.5 w-3.5 shrink-0 opacity-70" aria-hidden />
          <span className="hidden sm:inline">Home</span>
        </button>
        {parent ? (
          <>
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-slate-600" aria-hidden />
            <button
              type="button"
              data-testid="breadcrumb-parent"
              onClick={() => onNavigate(parent.view)}
              className="truncate rounded-lg px-2 py-1.5 text-sm text-slate-400 transition hover:bg-white/[0.04] hover:text-slate-100"
            >
              {parent.label}
            </button>
          </>
        ) : null}
        <ChevronRight className="h-3.5 w-3.5 shrink-0 text-slate-600" aria-hidden />
        <span className="truncate px-1 py-1.5 text-sm font-medium text-slate-100">{label}</span>
        {workload ? (
          <>
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-slate-600" aria-hidden />
            <button
              type="button"
              data-testid="breadcrumb-workload"
              onClick={() => navigate(pathWithQuery(viewToPath(currentView), { workload }))}
              className="truncate max-w-[12rem] rounded-lg px-2 py-1.5 text-sm font-mono text-aether hover:bg-white/[0.04] hover:underline"
            >
              {workload}
            </button>
          </>
        ) : null}
      </div>
    </nav>
  );
}

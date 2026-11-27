// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { ChevronRight, Home, Star } from 'lucide-react';
import { useNavigate } from 'react-router';
import type { AppView } from '../types/api';
import { VIEW_LABELS, getViewMeta } from '../utils/dashboardNav';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import { getParentView, getSectionLabel } from '../utils/navSections';
import { usePreferences } from '../hooks/usePreferences';
import { cn } from '../lib/cn';

interface BreadcrumbProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  workloadName?: string;
}

const crumb = 'truncate rounded-md px-1.5 py-0.5';
const crumbLink = `${crumb} text-muted transition-colors hover:bg-hover hover:text-foreground`;
const separator = <ChevronRight className="h-3.5 w-3.5 shrink-0 text-subtle" aria-hidden />;

/** Where you are: Home, section or parent page, this page, and the focused workload. */
export default function Breadcrumb({ currentView, onNavigate, workloadName }: BreadcrumbProps) {
  const navigate = useNavigate();
  const { pinnedViews, togglePin } = usePreferences();
  if (currentView === 'overview') return null;

  const label = VIEW_LABELS[currentView];
  const workload = workloadName?.trim();
  const parent = getParentView(currentView);
  const pinned = pinnedViews.includes(currentView);

  return (
    <nav className="dash-breadcrumb mb-2" aria-label="Breadcrumb">
      <div className="flex min-w-0 flex-wrap items-center gap-0.5 text-[13px]">
        <button type="button" onClick={() => onNavigate('overview')} aria-label="Home" className={`${crumbLink} inline-flex items-center`}>
          <Home className="h-3.5 w-3.5 shrink-0" aria-hidden />
        </button>
        {separator}
        {parent ? (
          <button type="button" data-testid="breadcrumb-parent" onClick={() => onNavigate(parent)} className={crumbLink}>
            {getViewMeta(parent).breadcrumb}
          </button>
        ) : (
          <span data-testid="breadcrumb-group" className={`${crumb} text-subtle`}>
            {getSectionLabel(currentView)}
          </span>
        )}
        {separator}
        <span className={`${crumb} font-medium text-foreground`} aria-current="page">
          {label}
        </span>
        {workload ? (
          <>
            {separator}
            <button
              type="button"
              data-testid="breadcrumb-workload"
              onClick={() => navigate(pathWithQuery(viewToPath(currentView), { workload }))}
              className={`${crumb} max-w-[12rem] font-mono text-primary hover:bg-hover hover:underline`}
            >
              {workload}
            </button>
          </>
        ) : null}
        <button
          type="button"
          onClick={() => togglePin(currentView)}
          aria-pressed={pinned}
          aria-label={pinned ? 'Unpin this page' : 'Pin this page'}
          title={pinned ? 'Unpin this page ( p )' : 'Pin this page to the sidebar ( p )'}
          className={cn('ml-1 grid h-6 w-6 place-items-center rounded-md hover:bg-hover', pinned ? 'text-primary' : 'text-subtle hover:text-foreground')}
        >
          <Star className={cn('h-3.5 w-3.5', pinned && 'fill-current')} aria-hidden />
        </button>
      </div>
    </nav>
  );
}

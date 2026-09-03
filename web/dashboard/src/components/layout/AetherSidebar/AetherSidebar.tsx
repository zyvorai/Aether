// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { ChevronDown, ChevronLeft, ChevronRight, Search, X } from 'lucide-react';
import type { AppView } from '../../../types/api';
import { getViewMeta, type DashboardViewMeta } from '../../../utils/dashboardNav';
import { SIDEBAR_PRIMARY, SIDEBAR_SECTIONS, type SidebarSection } from '../../../utils/sidebarNav';
import { cn } from '../../../lib/cn';

const COLLAPSE_KEY = 'aether-sidebar-collapsed';
const SECTIONS_COLLAPSED_KEY = 'aether-sidebar-sections-collapsed';

export function loadSidebarCollapsed(): boolean {
  try {
    return localStorage.getItem(COLLAPSE_KEY) === 'true';
  } catch {
    return false;
  }
}

export function saveSidebarCollapsed(collapsed: boolean) {
  try {
    localStorage.setItem(COLLAPSE_KEY, collapsed ? 'true' : 'false');
  } catch {
    /* ignore */
  }
}

function loadCollapsedSections(): Set<string> {
  try {
    const raw = localStorage.getItem(SECTIONS_COLLAPSED_KEY);
    if (!raw) return new Set();
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) ? new Set(parsed.filter((x): x is string => typeof x === 'string')) : new Set();
  } catch {
    return new Set();
  }
}

function saveCollapsedSections(collapsed: Set<string>) {
  try {
    localStorage.setItem(SECTIONS_COLLAPSED_KEY, JSON.stringify([...collapsed]));
  } catch {
    /* ignore */
  }
}

interface AetherSidebarProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  collapsed: boolean;
  onCollapsedChange: (collapsed: boolean) => void;
  mobile?: boolean;
  onNavigateMobile?: () => void;
}

function SidebarLink({
  item,
  active,
  rail,
  onNavigate,
  className,
}: {
  item: DashboardViewMeta;
  active: boolean;
  rail: boolean;
  onNavigate: (view: AppView) => void;
  className?: string;
}) {
  const Icon = item.icon;
  return (
    <button
      type="button"
      title={item.label}
      aria-current={active ? 'page' : undefined}
      onClick={() => onNavigate(item.view)}
      className={cn(
        'flex items-center gap-2 rounded-md px-2 py-1.5 text-[13px] leading-none transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/50',
        active ? 'bg-brand-wash text-brand' : 'text-ink-2 hover:bg-hover hover:text-ink',
        rail ? 'w-full justify-center px-1.5 py-1.5' : 'w-full',
        className,
      )}
    >
      <Icon className="h-4 w-4 shrink-0" strokeWidth={1.6} aria-hidden />
      {rail ? <span className="sr-only">{item.label}</span> : <span className="truncate">{item.label}</span>}
    </button>
  );
}

function SidebarSectionBlock({
  section,
  currentView,
  rail,
  expanded,
  onToggleExpanded,
  onNavigate,
}: {
  section: SidebarSection;
  currentView: AppView;
  rail: boolean;
  expanded: boolean;
  onToggleExpanded: () => void;
  onNavigate: (view: AppView) => void;
}) {
  const hasActiveItem = section.items.some((item) => item.view === currentView);
  const [flyoutOpen, setFlyoutOpen] = useState(false);
  const [flyoutPos, setFlyoutPos] = useState({ top: 0, left: 0 });
  const triggerRef = useRef<HTMLButtonElement>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout>>();
  const clearCloseTimer = useCallback(() => {
    if (closeTimer.current) clearTimeout(closeTimer.current);
  }, []);
  const scheduleClose = useCallback(() => {
    clearCloseTimer();
    closeTimer.current = setTimeout(() => setFlyoutOpen(false), 120);
  }, [clearCloseTimer]);
  const openFlyout = useCallback(() => {
    clearCloseTimer();
    const rect = triggerRef.current?.getBoundingClientRect();
    if (rect) setFlyoutPos({ top: rect.top, left: rect.right + 8 });
    setFlyoutOpen(true);
  }, [clearCloseTimer]);
  useEffect(() => () => clearCloseTimer(), [clearCloseTimer]);

  if (rail) {
    return (
      <div className="relative" onMouseEnter={openFlyout} onMouseLeave={scheduleClose}>
        <button
          ref={triggerRef}
          type="button"
          title={section.label}
          aria-haspopup="menu"
          aria-expanded={flyoutOpen}
          onFocus={openFlyout}
          onBlur={scheduleClose}
          className={cn(
            'mx-auto my-0.5 flex h-3 w-full items-center justify-center rounded-md text-ink-3 hover:bg-hover hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/50',
            hasActiveItem ? 'text-brand' : '',
          )}
        >
          <span className="h-px w-6 bg-border" aria-hidden />
        </button>
        {flyoutOpen && typeof document !== 'undefined'
          ? createPortal(
              <div
                role="menu"
                aria-label={section.label}
                onMouseEnter={clearCloseTimer}
                onMouseLeave={scheduleClose}
                style={{ position: 'fixed', top: flyoutPos.top, left: flyoutPos.left }}
                className="z-[70] min-w-[190px] max-h-[min(70vh,420px)] overflow-y-auto rounded-lg border border-border bg-raised py-1.5 shadow-lg"
              >
                <p className="px-3 pb-1 text-[11px] font-medium uppercase tracking-wide text-ink-3">{section.label}</p>
                <ul className="px-1">
                  {section.items.map((item) => (
                    <li key={item.view}>
                      <SidebarLink item={item} active={item.view === currentView} rail={false} onNavigate={onNavigate} />
                    </li>
                  ))}
                </ul>
              </div>,
              document.body,
            )
          : null}
      </div>
    );
  }

  return (
    <div className="mb-2">
      <button
        type="button"
        aria-expanded={expanded}
        onClick={onToggleExpanded}
        className="flex w-full items-center gap-1 px-2 py-1 text-left text-[11px] font-medium uppercase tracking-wide text-ink-3 hover:text-ink-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/50"
      >
        <ChevronDown className={cn('h-3 w-3 shrink-0 transition-transform', expanded ? '' : '-rotate-90')} aria-hidden />
        <span className="truncate">{section.label}</span>
      </button>
      {expanded ? (
        <ul className="space-y-px">
          {section.items.map((item) => (
            <li key={item.view}>
              <SidebarLink item={item} active={item.view === currentView} rail={false} onNavigate={onNavigate} />
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

export default function AetherSidebar({
  currentView,
  onNavigate,
  collapsed,
  onCollapsedChange,
  mobile = false,
  onNavigateMobile,
}: AetherSidebarProps) {
  const [filter, setFilter] = useState('');
  const [collapsedSections, setCollapsedSections] = useState<Set<string>>(() => loadCollapsedSections());
  const filterNorm = filter.trim().toLowerCase();
  const rail = !mobile && collapsed;

  const activeSectionId = useMemo(() => getViewMeta(currentView).group, [currentView]);

  useEffect(() => {
    if (activeSectionId === 'primary') return;
    setCollapsedSections((prev) => {
      if (!prev.has(activeSectionId)) return prev;
      const next = new Set(prev);
      next.delete(activeSectionId);
      saveCollapsedSections(next);
      return next;
    });
  }, [activeSectionId]);

  const toggleSection = useCallback((id: string) => {
    setCollapsedSections((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      saveCollapsedSections(next);
      return next;
    });
  }, []);

  const handleNavigate = useCallback((view: AppView) => {
    onNavigate(view);
    onNavigateMobile?.();
  }, [onNavigate, onNavigateMobile]);

  const forceOpen = Boolean(filterNorm);
  const filteredPrimary = filterNorm
    ? SIDEBAR_PRIMARY.filter((item) => item.label.toLowerCase().includes(filterNorm))
    : SIDEBAR_PRIMARY;
  const filteredSections = filterNorm
    ? SIDEBAR_SECTIONS.map((section) => ({
        ...section,
        items: section.items.filter((item) => item.label.toLowerCase().includes(filterNorm)),
      })).filter((section) => section.items.length > 0)
    : SIDEBAR_SECTIONS;

  const widthClass = mobile ? 'w-72' : rail ? 'w-16' : 'w-64';

  return (
    <aside
      className={cn(
        'flex flex-shrink-0 flex-col bg-canvas border-r border-border',
        mobile ? 'h-full w-72' : `hidden lg:flex ${widthClass}`,
      )}
      aria-label="Aether navigation"
    >
      {!rail ? (
        <div className="px-2.5 pb-1.5 pt-2.5">
          <label className="relative block">
            <Search className="pointer-events-none absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-ink-3" aria-hidden />
            <input
              type="search"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filter"
              aria-label="Filter navigation"
              className="w-full min-w-0 rounded-md bg-hover py-1.5 pl-7 pr-7 text-[13px] text-ink placeholder:text-ink-3 focus:outline-none focus:ring-1 focus:ring-brand/30"
            />
            {filter ? (
              <button
                type="button"
                onClick={() => setFilter('')}
                aria-label="Clear filter"
                className="absolute right-2 top-1/2 -translate-y-1/2 text-ink-2 hover:text-ink"
              >
                <X className="h-3 w-3" />
              </button>
            ) : null}
          </label>
        </div>
      ) : null}

      <nav className={cn('flex-1 overflow-y-auto overflow-x-hidden min-h-0 px-2', rail ? 'py-1.5' : 'py-2')} aria-label="Primary">
        <ul className={cn('mb-2', rail ? 'space-y-0' : 'space-y-px')}>
          {filteredPrimary.map((item) => (
            <li key={item.view}>
              <SidebarLink item={item} active={item.view === currentView} rail={rail} onNavigate={handleNavigate} />
            </li>
          ))}
        </ul>
        {filteredSections.map((section) => (
          <SidebarSectionBlock
            key={section.id}
            section={section}
            currentView={currentView}
            rail={rail}
            expanded={forceOpen || !collapsedSections.has(section.id)}
            onToggleExpanded={() => toggleSection(section.id)}
            onNavigate={handleNavigate}
          />
        ))}
        {forceOpen && filteredPrimary.length === 0 && filteredSections.length === 0 ? (
          <p className="px-2 py-6 text-sm text-ink-2">No matches for "{filter}".</p>
        ) : null}
      </nav>

      {!mobile ? (
        <div className="border-t border-border p-1.5">
          <button
            type="button"
            onClick={() => onCollapsedChange(!collapsed)}
            aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            className="flex w-full items-center justify-center gap-1.5 rounded-md py-1.5 text-[11px] text-ink-3 hover:bg-hover hover:text-ink-2 transition-colors"
          >
            {collapsed ? <ChevronRight className="h-3.5 w-3.5" /> : <><ChevronLeft className="h-3.5 w-3.5" /><span>Collapse</span></>}
          </button>
        </div>
      ) : null}
    </aside>
  );
}

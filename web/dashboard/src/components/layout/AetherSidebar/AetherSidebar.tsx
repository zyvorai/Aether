// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState } from 'react';
import { ChevronDown, Search, Sparkles, X } from 'lucide-react';
import type { AppView } from '../../../types/api';
import { getViewMeta, type DashboardViewMeta } from '../../../utils/dashboardNav';
import { SIDEBAR_PRIMARY, SIDEBAR_SECTIONS, type SidebarSection } from '../../../utils/sidebarNav';
import { cn } from '../../../lib/cn';

const SECTIONS_COLLAPSED_KEY = 'aether-sidebar-sections-collapsed';

function loadCollapsedSections(): Set<string> {
  try {
    const raw = localStorage.getItem(SECTIONS_COLLAPSED_KEY);
    // Default: all sections collapsed (Apple-lite). Only expand what the user opened.
    if (!raw) return new Set(['intelligence', 'operations', 'resources']);
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) ? new Set(parsed.filter((x): x is string => typeof x === 'string')) : new Set(['intelligence', 'operations', 'resources']);
  } catch {
    return new Set(['intelligence', 'operations', 'resources']);
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
  mobile?: boolean;
  onNavigateMobile?: () => void;
}

function SidebarLink({
  item,
  active,
  onNavigate,
  className,
}: {
  item: DashboardViewMeta;
  active: boolean;
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
        'relative flex w-full items-center gap-2 rounded-[var(--radius-md)] px-2 py-1.5 text-[13px] leading-none tracking-[-0.005em] transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50',
        active ? 'glass-fill font-medium text-primary shadow-card' : 'text-muted hover:bg-hover hover:text-foreground',
        className,
      )}
    >
      <Icon className="h-4 w-4 shrink-0" strokeWidth={1.6} aria-hidden />
      <span className="truncate">{item.label}</span>
    </button>
  );
}

function SidebarSectionBlock({
  section,
  currentView,
  expanded,
  onToggleExpanded,
  onNavigate,
}: {
  section: SidebarSection;
  currentView: AppView;
  expanded: boolean;
  onToggleExpanded: () => void;
  onNavigate: (view: AppView) => void;
}) {
  return (
    <div className="mb-2">
      <button
        type="button"
        aria-expanded={expanded}
        onClick={onToggleExpanded}
        className="flex w-full items-center gap-1 px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-[var(--tracking-eyebrow)] text-subtle hover:text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
      >
        <ChevronDown className={cn('h-3 w-3 shrink-0 transition-transform', expanded ? '' : '-rotate-90')} aria-hidden />
        <span className="truncate">{section.label}</span>
      </button>
      {expanded ? (
        <ul className="space-y-px">
          {section.items.map((item) => (
            <li key={item.view}>
              <SidebarLink item={item} active={item.view === currentView} onNavigate={onNavigate} />
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
  mobile = false,
  onNavigateMobile,
}: AetherSidebarProps) {
  const [filter, setFilter] = useState('');
  const [collapsedSections, setCollapsedSections] = useState<Set<string>>(() => loadCollapsedSections());
  const filterNorm = filter.trim().toLowerCase();

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

  return (
    <aside
      className={cn(
        'flex flex-shrink-0 flex-col',
        mobile ? 'h-full w-72 glass' : 'hidden lg:flex w-64 m-3.5 rounded-[var(--radius-2xl)] glass-fill',
      )}
      aria-label="Aether navigation"
    >
      <div className="px-2.5 pb-1.5 pt-2.5">
        <label className="relative block">
          <Search className="pointer-events-none absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-subtle" aria-hidden />
          <input
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter"
            aria-label="Filter navigation"
            className="glass-fill w-full min-w-0 rounded-md py-1.5 pl-7 pr-7 text-[13px] text-foreground placeholder:text-subtle focus:outline-none focus:ring-1 focus:ring-primary/30"
          />
          {filter ? (
            <button
              type="button"
              onClick={() => setFilter('')}
              aria-label="Clear filter"
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted hover:text-foreground"
            >
              <X className="h-3 w-3" />
            </button>
          ) : null}
        </label>
      </div>

      <nav className="flex-1 overflow-y-auto overflow-x-hidden min-h-0 px-2 py-2" aria-label="Primary">
        <ul className="mb-2 space-y-px">
          {filteredPrimary.map((item) => (
            <li key={item.view}>
              <SidebarLink item={item} active={item.view === currentView} onNavigate={handleNavigate} />
            </li>
          ))}
        </ul>
        {filteredSections.map((section) => (
          <SidebarSectionBlock
            key={section.id}
            section={section}
            currentView={currentView}
            expanded={forceOpen || !collapsedSections.has(section.id)}
            onToggleExpanded={() => toggleSection(section.id)}
            onNavigate={handleNavigate}
          />
        ))}
        {forceOpen && filteredPrimary.length === 0 && filteredSections.length === 0 ? (
          <p className="px-2 py-6 text-sm text-muted">No matches for "{filter}".</p>
        ) : null}
      </nav>

      <div className="border-t border-border p-1.5">
        <button
          type="button"
          onClick={() => handleNavigate('zyra')}
          title="Ask Zyra"
          className="flex w-full items-center gap-2.5 rounded-[var(--radius-lg)] px-2.5 py-2 text-left transition-colors hover:bg-hover"
          style={{ background: 'linear-gradient(135deg, rgba(168,98,234,.16), rgba(249,115,22,.14))' }}
        >
          <span
            className="flex h-[26px] w-[26px] shrink-0 items-center justify-center rounded-[var(--radius-sm)] text-white"
            style={{ background: 'linear-gradient(135deg, #a862ea, #f97316)' }}
          >
            <Sparkles className="h-3.5 w-3.5" aria-hidden />
          </span>
          <span className="min-w-0">
            <span className="block text-[12.5px] font-semibold text-foreground">Ask Zyra</span>
            <span className="block truncate text-[11px] text-subtle">AI infrastructure copilot</span>
          </span>
        </button>
      </div>
    </aside>
  );
}

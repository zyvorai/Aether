// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from 'react';
import { Link, useNavigate } from 'react-router';
import { ChevronDown, PanelLeftClose, PanelLeftOpen, Search, Sparkles, Star, X } from 'lucide-react';
import type { AppView } from '../../../types/api';
import { getViewMeta, type DashboardViewMeta } from '../../../utils/dashboardNav';
import {
  buildNavTree,
  filterViews,
  getChildViews,
  getParentView,
  getRootView,
  getSectionId,
  type NavSection,
  type NavSectionId,
} from '../../../utils/navSections';
import { usePreferences } from '../../../hooks/usePreferences';
import { cn } from '../../../lib/cn';

const SECTIONS_KEY = 'aether-sidebar-open-sections';
const DEFAULT_OPEN: NavSectionId[] = ['workspace', 'observe'];
const NAV_TREE = buildNavTree();

function loadOpenSections(): Set<NavSectionId> {
  try {
    const raw = localStorage.getItem(SECTIONS_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : null;
    if (Array.isArray(parsed)) return new Set(parsed.filter((x): x is NavSectionId => typeof x === 'string'));
  } catch {
    /* fall through to the default */
  }
  return new Set(DEFAULT_OPEN);
}

function saveOpenSections(open: Set<NavSectionId>) {
  try {
    localStorage.setItem(SECTIONS_KEY, JSON.stringify([...open]));
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

/** Modified clicks keep the browser behaviour (open in a new tab or window). */
function isPlainClick(event: MouseEvent): boolean {
  return event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey;
}

interface PageLinkProps {
  item: DashboardViewMeta;
  active: boolean;
  onNavigate: (view: AppView) => void;
  nested?: boolean;
  hint?: string;
  pinned?: boolean;
  onTogglePin?: (view: AppView) => void;
}

function PageLink({ item, active, onNavigate, nested, hint, pinned, onTogglePin }: PageLinkProps) {
  const Icon = item.icon;
  return (
    <div className="side-row">
      <Link
        to={item.path}
        title={item.subtitle}
        aria-current={active ? 'page' : undefined}
        className={cn('side-link', nested && 'side-link-nested', active && 'side-link-active', onTogglePin && 'side-link-pinnable')}
        onClick={(event) => {
          if (!isPlainClick(event)) return;
          event.preventDefault();
          onNavigate(item.view);
        }}
      >
        {nested ? null : <Icon className="h-4 w-4 shrink-0" strokeWidth={1.6} aria-hidden />}
        <span className="truncate">{item.label}</span>
        {hint ? <span className="side-link-hint">{hint}</span> : null}
      </Link>
      {onTogglePin ? (
        <button
          type="button"
          className="side-pin"
          data-pinned={pinned ? 'true' : undefined}
          aria-pressed={pinned}
          aria-label={`${pinned ? 'Unpin' : 'Pin'} ${item.label}`}
          title={pinned ? 'Unpin page' : 'Pin page'}
          onClick={() => onTogglePin(item.view)}
        >
          <Star className={cn('h-3.5 w-3.5', pinned && 'fill-current')} aria-hidden />
        </button>
      ) : null}
    </div>
  );
}

export default function AetherSidebar({ currentView, onNavigate, mobile = false, onNavigateMobile }: AetherSidebarProps) {
  const navigate = useNavigate();
  const { pinnedViews, sidebarCollapsed, setPreferences, togglePin } = usePreferences();
  const [filter, setFilter] = useState('');
  const [openSections, setOpenSections] = useState<Set<NavSectionId>>(loadOpenSections);
  const filterRef = useRef<HTMLInputElement>(null);
  const navRef = useRef<HTMLElement>(null);
  const collapsed = sidebarCollapsed && !mobile;

  const activeSection = useMemo(() => getSectionId(currentView), [currentView]);
  const [arrivedIn, setArrivedIn] = useState<NavSectionId | null>(activeSection);
  const rootView = useMemo(() => getRootView(currentView), [currentView]);

  // The section holding the current page opens when you arrive on it. That is
  // not saved, so browsing around does not leave every section open.
  useEffect(() => setArrivedIn(activeSection), [activeSection]);

  // Keep the current page visible in a long list.
  useEffect(() => {
    const nav = navRef.current;
    const active = nav?.querySelector<HTMLElement>('[aria-current="page"]');
    if (!nav || !active) return;
    const frame = nav.getBoundingClientRect();
    const box = active.getBoundingClientRect();
    if (box.top < frame.top || box.bottom > frame.bottom) {
      nav.scrollTop += box.top - frame.top - frame.height / 3;
    }
  }, [currentView, collapsed, arrivedIn]);

  const isOpen = (id: NavSectionId) => openSections.has(id) || arrivedIn === id;

  const toggleSection = (id: NavSectionId) => {
    const next = new Set(openSections);
    if (isOpen(id)) {
      next.delete(id);
      if (arrivedIn === id) setArrivedIn(null);
    } else {
      next.add(id);
    }
    saveOpenSections(next);
    setOpenSections(next);
  };

  const go = useCallback(
    (view: AppView) => {
      onNavigate(view);
      onNavigateMobile?.();
    },
    [onNavigate, onNavigateMobile],
  );

  const expandTo = (section?: NavSectionId) => {
    setPreferences({ sidebarCollapsed: false });
    if (section) {
      setOpenSections((prev) => {
        const next = new Set(prev).add(section);
        saveOpenSections(next);
        return next;
      });
    }
  };

  // A stored pin can name a page this build no longer has.
  const pinned = pinnedViews.flatMap((view) => {
    try {
      return [getViewMeta(view)];
    } catch {
      return [];
    }
  });
  const results = filter.trim() ? filterViews(filter) : null;

  const onFilterKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Escape' && filter) {
      event.stopPropagation();
      setFilter('');
    } else if (event.key === 'Enter' && results?.[0]) {
      event.preventDefault();
      setFilter('');
      navigate(results[0].path);
      onNavigateMobile?.();
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      navRef.current?.querySelector<HTMLElement>('a, button')?.focus();
    }
  };

  const onNavKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    const items = Array.from(navRef.current?.querySelectorAll<HTMLElement>('a, button.side-section') ?? []);
    const index = items.indexOf(document.activeElement as HTMLElement);
    if (index < 0) return;
    event.preventDefault();
    items[index + (event.key === 'ArrowDown' ? 1 : -1)]?.focus();
  };

  if (collapsed) {
    const workspace = (NAV_TREE[0]?.items ?? []).filter((item) => !pinnedViews.includes(item.view));
    return (
      <aside className="side-shell side-docked side-rail hidden lg:flex" aria-label="Aether navigation">
        <button
          type="button"
          className="side-rail-item mt-2"
          onClick={() => expandTo()}
          aria-label="Expand navigation"
          title="Expand navigation ( [ )"
        >
          <PanelLeftOpen className="h-4 w-4" aria-hidden />
        </button>
        <nav ref={navRef} className="flex min-h-0 flex-1 flex-col items-center gap-1 overflow-y-auto py-1" aria-label="Primary">
          {[...pinned, ...workspace].map((item) => {
            const Icon = item.icon;
            return (
              <Link
                key={item.view}
                to={item.path}
                className={cn('side-rail-item', item.view === rootView && 'side-link-active')}
                aria-current={item.view === currentView ? 'page' : undefined}
                aria-label={item.label}
                title={item.label}
              >
                <Icon className="h-4 w-4" strokeWidth={1.7} aria-hidden />
              </Link>
            );
          })}
          <span className="side-rail-rule" aria-hidden />
          {NAV_TREE.slice(1).map((section) => {
            const Icon = section.items[0].icon;
            return (
              <button
                key={section.id}
                type="button"
                className={cn('side-rail-item', section.id === activeSection && 'side-link-active')}
                aria-label={`Open ${section.label}`}
                title={section.label}
                onClick={() => expandTo(section.id)}
              >
                <Icon className="h-4 w-4" strokeWidth={1.7} aria-hidden />
              </button>
            );
          })}
        </nav>
        <Link to="/zyra" className="side-rail-item mb-2" aria-label="Ask Zyra" title="Ask Zyra">
          <Sparkles className="h-4 w-4" aria-hidden />
        </Link>
      </aside>
    );
  }

  const renderPage = (item: DashboardViewMeta) => {
    const children = item.view === rootView ? getChildViews(item.view) : [];
    return (
      <li key={item.view}>
        <PageLink
          item={item}
          active={item.view === currentView}
          onNavigate={go}
          pinned={pinnedViews.includes(item.view)}
          onTogglePin={togglePin}
        />
        {children.length > 0 ? (
          <ul className="side-children" aria-label={`${item.label} pages`}>
            {children.map((child) => (
              <li key={child.view}>
                <PageLink item={child} active={child.view === currentView} onNavigate={go} nested />
              </li>
            ))}
          </ul>
        ) : null}
      </li>
    );
  };

  const renderSection = (section: NavSection) => {
    const open = isOpen(section.id);
    return (
      <div key={section.id} className="mb-2">
        <button type="button" aria-expanded={open} onClick={() => toggleSection(section.id)} className="side-section">
          <span className="truncate">{section.label}</span>
          <ChevronDown className={cn('h-3 w-3 shrink-0 transition-transform', open ? '' : '-rotate-90')} aria-hidden />
        </button>
        {open ? <ul className="space-y-px">{section.items.map(renderPage)}</ul> : null}
      </div>
    );
  };

  return (
    <aside
      className={cn(
        'side-shell flex',
        mobile ? 'h-full w-[min(18rem,86vw)] bg-surface-elevated shadow-ambient' : 'side-docked hidden w-64 lg:flex',
      )}
      aria-label="Aether navigation"
    >
      <div className="flex items-center gap-1 px-3 pb-1 pt-3">
        <label className="relative block min-w-0 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-subtle" aria-hidden />
          <input
            ref={filterRef}
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            onKeyDown={onFilterKeyDown}
            placeholder="Filter pages"
            aria-label="Filter navigation"
            className="side-filter"
          />
          {filter ? (
            <button
              type="button"
              onClick={() => {
                setFilter('');
                filterRef.current?.focus();
              }}
              aria-label="Clear filter"
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted hover:text-foreground"
            >
              <X className="h-3 w-3" aria-hidden />
            </button>
          ) : null}
        </label>
        {mobile ? null : (
          <button
            type="button"
            className="side-rail-item"
            onClick={() => setPreferences({ sidebarCollapsed: true })}
            aria-label="Collapse navigation"
            title="Collapse navigation ( [ )"
          >
            <PanelLeftClose className="h-4 w-4" aria-hidden />
          </button>
        )}
      </div>

      <nav ref={navRef} onKeyDown={onNavKeyDown} className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-3 py-2" aria-label="Primary">
        {results ? (
          results.length > 0 ? (
            <ul className="space-y-px">
              {results.map((item) => {
                const parent = getParentView(item.view);
                return (
                  <li key={item.view}>
                    <PageLink
                      item={item}
                      active={item.view === currentView}
                      onNavigate={(view) => {
                        setFilter('');
                        go(view);
                      }}
                      hint={parent ? getViewMeta(parent).label : undefined}
                    />
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="px-2 py-6 text-sm text-muted" role="status">
              No page matches "{filter}".
            </p>
          )
        ) : (
          <>
            {pinned.length > 0 ? (
              <div className="mb-2">
                <p className="side-section side-section-static">Pinned</p>
                <ul className="space-y-px">
                  {pinned.map((item) => (
                    <li key={item.view}>
                      <PageLink item={item} active={item.view === currentView} onNavigate={go} pinned onTogglePin={togglePin} />
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}
            {NAV_TREE.map(renderSection)}
          </>
        )}
      </nav>

      <div className="border-t border-border p-3">
        <button type="button" onClick={() => go('zyra')} title="Ask Zyra" className="side-assistant">
          <span className="flex h-[26px] w-[26px] shrink-0 items-center justify-center rounded-lg bg-accent-fill text-white">
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

// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/**
 * Information architecture: which section each page lives in and how pages nest.
 *
 * Everything is derived from DASHBOARD_VIEWS, so a newly registered page shows
 * up without editing this file:
 * - a page whose path is under another page's path (`/security/sbom` under
 *   `/security`) becomes that page's child;
 * - a top-level page that no section lists falls back to a section chosen by its
 *   `group`, unless it is marked `sidebar: false`.
 * List a view in NAV_SECTIONS only to choose a better section or order.
 */

import { DASHBOARD_VIEWS, getViewMeta, type DashboardViewMeta, type NavGroup } from './dashboardNav';
import type { AppView } from '../types/api';

export type NavSectionId =
  | 'workspace'
  | 'build'
  | 'observe'
  | 'infrastructure'
  | 'intelligence'
  | 'security'
  | 'platform';

interface NavSectionDefinition {
  id: NavSectionId;
  label: string;
  views: AppView[];
}

export const NAV_SECTIONS: NavSectionDefinition[] = [
  { id: 'workspace', label: 'Workspace', views: ['overview', 'workloads', 'applications', 'fabric'] },
  { id: 'build', label: 'Build and deploy', views: ['editor', 'compose', 'templates', 'helm', 'gitops', 'migrations', 'envs', 'plugins'] },
  { id: 'observe', label: 'Observe', views: ['observability', 'health', 'events', 'alerts', 'metrics', 'activity', 'sla'] },
  { id: 'infrastructure', label: 'Infrastructure', views: ['clusters', 'fleet', 'scheduler', 'deps', 'forge', 'storage', 'backups'] },
  { id: 'intelligence', label: 'Intelligence', views: ['intelligence', 'ai', 'zyra', 'affinity', 'drift', 'cost'] },
  { id: 'security', label: 'Security', views: ['security', 'policy', 'secrets', 'rbac', 'audit'] },
  { id: 'platform', label: 'Platform', views: ['platform', 'settings', 'hosted', 'openapi', 'labs'] },
];

const GROUP_FALLBACK: Record<NavGroup, NavSectionId> = {
  primary: 'workspace',
  intelligence: 'intelligence',
  operations: 'observe',
  resources: 'platform',
};

const LISTED = new Map<AppView, NavSectionId>(
  NAV_SECTIONS.flatMap((section) => section.views.map((view) => [view, section.id] as const)),
);

const BY_PATH = new Map(DASHBOARD_VIEWS.map((meta) => [meta.path, meta]));

/** The page this one nests under, or null for a top-level page. */
export function getParentView(view: AppView): AppView | null {
  const segments = getViewMeta(view).path.split('/').filter(Boolean);
  while (segments.length > 1) {
    segments.pop();
    const parent = BY_PATH.get(`/${segments.join('/')}`);
    if (parent) return parent.view;
  }
  return null;
}

export function getChildViews(view: AppView): DashboardViewMeta[] {
  return DASHBOARD_VIEWS.filter((meta) => getParentView(meta.view) === view);
}

/** The top-level page a view belongs to (itself when it is top-level). */
export function getRootView(view: AppView): AppView {
  let current = view;
  for (let parent = getParentView(current); parent; parent = getParentView(current)) current = parent;
  return current;
}

export function getSectionId(view: AppView): NavSectionId {
  const root = getRootView(view);
  return LISTED.get(root) ?? GROUP_FALLBACK[getViewMeta(root).group];
}

export function getSectionLabel(view: AppView): string {
  const id = getSectionId(view);
  return NAV_SECTIONS.find((section) => section.id === id)?.label ?? 'Workspace';
}

export interface NavSection {
  id: NavSectionId;
  label: string;
  items: DashboardViewMeta[];
}

/** Sections with their top-level pages, in display order. */
export function buildNavTree(views: DashboardViewMeta[] = DASHBOARD_VIEWS): NavSection[] {
  return NAV_SECTIONS.map((section) => {
    const listed = section.views.map((view) => views.find((meta) => meta.view === view));
    const unlisted = views.filter(
      (meta) =>
        !LISTED.has(meta.view) &&
        meta.sidebar !== false &&
        getParentView(meta.view) === null &&
        GROUP_FALLBACK[meta.group] === section.id,
    );
    return {
      id: section.id,
      label: section.label,
      items: [...listed.filter((meta): meta is DashboardViewMeta => Boolean(meta)), ...unlisted],
    };
  }).filter((section) => section.items.length > 0);
}

/**
 * The tabs shown above a page that has sub-pages: the parent first, then its
 * children. Empty when the page stands alone.
 */
export function getSectionTabs(view: AppView): DashboardViewMeta[] {
  const parent = getParentView(view) ?? view;
  const children = getChildViews(parent);
  return children.length > 0 ? [getViewMeta(parent), ...children] : [];
}

/** Every page matching a filter, for the sidebar search. */
export function filterViews(query: string, views: DashboardViewMeta[] = DASHBOARD_VIEWS): DashboardViewMeta[] {
  const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return [];
  const scored = views.flatMap((meta) => {
    const name = `${meta.label} ${meta.paletteLabel ?? ''}`.toLowerCase();
    const parent = getParentView(meta.view);
    const context = `${parent ? getViewMeta(parent).label : ''} ${meta.subtitle} ${meta.path}`.toLowerCase();
    if (!terms.every((term) => name.includes(term) || context.includes(term))) return [];
    const inName = terms.filter((term) => name.includes(term)).length;
    const rank = inName * 10 + (name.startsWith(terms[0]) ? 5 : 0) + (parent ? 0 : 1);
    return [{ meta, rank }];
  });
  return scored.sort((a, b) => b.rank - a.rank || a.meta.label.localeCompare(b.meta.label)).map((entry) => entry.meta);
}

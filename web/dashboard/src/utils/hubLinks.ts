// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/**
 * Hub pages only list their sub-pages, so they cannot act on a query string.
 * A link or bookmark such as `/clusters?cluster=prod&kind=Pod` belongs to the
 * sub-page that reads those parameters. This decides which one, for every way
 * of arriving: in-app links, the command palette, deep links and bookmarks.
 */

import type { AppView } from '../types/api';
import { getViewMeta } from './dashboardNav';
import { viewToPath } from './dashboardRoutes';
import { getChildViews } from './navSections';

/** History state for a link that wants the hub list even though it carries parameters. */
export const SHOW_HUB_STATE = { showHub: true };

/** `?tab=` values that are not the last path segment of the sub-page showing them. */
const TAB_ALIASES: Partial<Record<AppView, Record<string, AppView>>> = {
  cost: { optimize: 'cost-intelligence' },
  intelligence: { place: 'intel-placement' },
  labs: { 'graph-search': 'labs-platform' },
};

/**
 * Per hub, which sub-page honours each filter when no `tab` names one. Order
 * matters when a link carries several. Hubs that are absent either show the
 * filter themselves (Fabric, Migrations) or have no filtered sub-page.
 */
export const HUB_FILTERS: Partial<Record<AppView, Record<string, AppView>>> = {
  affinity: { workload: 'affinity-recommend' },
  ai: { workload: 'ai-optimize' },
  alerts: { workload: 'alerts-rules' },
  clusters: {
    cluster: 'clusters-browse',
    namespace: 'clusters-browse',
    kind: 'clusters-browse',
    workload: 'clusters-browse',
  },
  cost: { workload: 'cost-estimate' },
  fleet: { cluster: 'fleet-overview', workload: 'fleet-placement' },
  gitops: { workload: 'gitops-sync' },
  intelligence: { workload: 'intel-predictions' },
  metrics: { workload: 'metrics-prometheus', q: 'metrics-prometheus' },
  platform: { workload: 'platform-runtime' },
  security: { workload: 'security-overview' },
};

function viewForTab(hub: AppView, tab: string): AppView | undefined {
  const alias = TAB_ALIASES[hub]?.[tab];
  if (alias) return alias;
  const path = `${getViewMeta(hub).path}/${tab}`;
  return getChildViews(hub).find((child) => child.path === path)?.view;
}

/** The sub-page a hub address should open instead, or null to stay where it is. */
export function hubTarget(view: AppView, search: string): AppView | null {
  if (getChildViews(view).length === 0) return null;
  const params = new URLSearchParams(search);
  // Sign-in reads `token` and removes it from the address first.
  if (params.has('token')) return null;

  const tab = params.get('tab');
  const byTab = tab ? viewForTab(view, tab) : undefined;
  if (byTab) return byTab;

  const filter = Object.entries(HUB_FILTERS[view] ?? {}).find(([name]) => params.get(name)?.trim());
  return filter ? filter[1] : null;
}

/** Where to replace the current address, keeping the whole query and fragment. */
export function hubRedirect(
  view: AppView,
  location: { search: string; hash?: string; state?: unknown },
): { pathname: string; search: string; hash: string } | null {
  if ((location.state as { showHub?: unknown } | null | undefined)?.showHub) return null;
  const target = hubTarget(view, location.search);
  if (!target) return null;
  return { pathname: viewToPath(target), search: location.search, hash: location.hash ?? '' };
}

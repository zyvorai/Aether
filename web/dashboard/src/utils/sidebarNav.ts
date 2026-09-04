// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { getViewMeta, type DashboardViewMeta, type NavGroup } from './dashboardNav';
import type { AppView } from '../types/api';

export interface SidebarSection {
  id: NavGroup;
  label: string;
  items: DashboardViewMeta[];
}

const SECTION_LABELS: Record<Exclude<NavGroup, 'primary'>, string> = {
  intelligence: 'Intelligence',
  operations: 'Operate',
  resources: 'Platform',
};

/** Apple-lite sidebar: hubs only. Everything else is command palette / hub children. */
const SIDEBAR_PRIMARY_VIEWS: AppView[] = ['overview', 'workloads', 'fabric'];
const SIDEBAR_SECTION_VIEWS: Record<Exclude<NavGroup, 'primary'>, AppView[]> = {
  intelligence: ['intelligence', 'confidential'],
  operations: ['observability', 'fleet', 'clusters', 'migrations'],
  resources: ['labs', 'settings'],
};

function metas(views: AppView[]): DashboardViewMeta[] {
  return views.map((view) => getViewMeta(view));
}

export const SIDEBAR_PRIMARY: DashboardViewMeta[] = metas(SIDEBAR_PRIMARY_VIEWS);

export const SIDEBAR_SECTIONS: SidebarSection[] = (
  ['intelligence', 'operations', 'resources'] as const
).map((id) => ({
  id,
  label: SECTION_LABELS[id],
  items: metas(SIDEBAR_SECTION_VIEWS[id]),
}));

export const ALL_SIDEBAR_ITEMS: DashboardViewMeta[] = [
  ...SIDEBAR_PRIMARY,
  ...SIDEBAR_SECTIONS.flatMap((section) => section.items),
];

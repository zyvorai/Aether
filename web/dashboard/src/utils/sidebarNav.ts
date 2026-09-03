// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { getViewsByGroup, type DashboardViewMeta, type NavGroup } from './dashboardNav';

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

export const SIDEBAR_PRIMARY: DashboardViewMeta[] = getViewsByGroup('primary');

export const SIDEBAR_SECTIONS: SidebarSection[] = (
  ['intelligence', 'operations', 'resources'] as const
).map((id) => ({ id, label: SECTION_LABELS[id], items: getViewsByGroup(id) }));

export const ALL_SIDEBAR_ITEMS: DashboardViewMeta[] = [
  ...SIDEBAR_PRIMARY,
  ...SIDEBAR_SECTIONS.flatMap((section) => section.items),
];

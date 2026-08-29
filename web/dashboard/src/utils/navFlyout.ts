import type { AppView } from '../types/api';
import { DASHBOARD_VIEWS, type DashboardViewMeta, type NavGroup } from './dashboardNav';

export interface NavFlyoutGroup {
  heading: string;
  links: DashboardViewMeta[];
  lead?: boolean;
}

export interface NavFlyoutPanel {
  key: Exclude<NavGroup, 'primary'>;
  label: string;
  groups: NavFlyoutGroup[];
}

const DIRECT_VIEWS: AppView[] = ['overview', 'applications', 'workloads', 'fabric'];

const GROUP_HEADINGS: Record<Exclude<NavGroup, 'primary'>, string[]> = {
  intelligence: ['Plan & optimize', 'Trust & policy', 'AI'],
  operations: ['Fleet', 'Operate', 'Build & deliver'],
  resources: ['Infrastructure', 'Configuration', 'Governance'],
};

function splitIntoColumns(items: DashboardViewMeta[], headings: string[]): NavFlyoutGroup[] {
  const size = Math.ceil(items.length / headings.length);
  return headings.map((heading, index) => ({
    heading,
    links: items.slice(index * size, (index + 1) * size),
    lead: index === 0,
  })).filter((group) => group.links.length > 0);
}

export const NAV_DIRECT_LINKS = DIRECT_VIEWS.map(
  (view) => DASHBOARD_VIEWS.find((item) => item.view === view)!,
);

export const NAV_FLYOUT_PANELS: NavFlyoutPanel[] = (
  ['intelligence', 'operations', 'resources'] as const
).map((group) => ({
  key: group,
  label: group[0].toUpperCase() + group.slice(1),
  groups: splitIntoColumns(
    DASHBOARD_VIEWS.filter((item) => item.group === group),
    GROUP_HEADINGS[group],
  ),
}));

/** The complete tree is shared by desktop flyouts and the mobile sheet. */
export const ALL_NAV_VIEWS = [...NAV_DIRECT_LINKS, ...NAV_FLYOUT_PANELS.flatMap(
  (panel) => panel.groups.flatMap((group) => group.links),
)];

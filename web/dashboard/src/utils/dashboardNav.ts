import type { AppView } from '../types/api';

export type NavGroup = 'primary' | 'intelligence' | 'operations' | 'resources';

export interface DashboardViewMeta {
  view: AppView;
  path: string;
  label: string;
  breadcrumb: string;
  subtitle: string;
  group: NavGroup;
  paletteLabel?: string;
}

export const DASHBOARD_VIEWS: DashboardViewMeta[] = [
  { view: 'overview', path: '/', label: 'Dashboard', breadcrumb: 'Dashboard', subtitle: 'Real-time view across all runtimes', group: 'primary' },
  { view: 'workloads', path: '/workloads', label: 'Workloads', breadcrumb: 'Workloads', subtitle: 'Deploy, monitor, and manage across Podman, Kubernetes, KubeVirt & Metal3', group: 'primary' },
  { view: 'ai', path: '/ai', label: 'AI Engine', breadcrumb: 'AI Engine', subtitle: 'Intent scoring, runtime recommendations & migration planning', group: 'intelligence' },
  { view: 'copilot', path: '/copilot', label: 'Ops Copilot', breadcrumb: 'Ops Copilot', subtitle: 'Natural language operations — health, drift, cost, and cluster queries', group: 'intelligence', paletteLabel: 'Copilot' },
  { view: 'cost', path: '/cost', label: 'Cost Estimation', breadcrumb: 'Cost Estimation', subtitle: 'Resource cost projections across runtimes', group: 'intelligence' },
  { view: 'affinity', path: '/affinity', label: 'Runtime Affinity', breadcrumb: 'Runtime Affinity', subtitle: 'Workload class affinity and runtime fit', group: 'intelligence' },
  { view: 'drift', path: '/drift', label: 'Drift Detection', breadcrumb: 'Drift Detection', subtitle: 'Configuration drift & desired-state reconciliation', group: 'intelligence' },
  { view: 'policy', path: '/policy', label: 'Policy Check', breadcrumb: 'Policy Check', subtitle: 'Validate workloads against policy rules', group: 'intelligence' },
  { view: 'clusters', path: '/clusters', label: 'Cluster Browser', breadcrumb: 'Cluster Browser', subtitle: 'Browse and manage Kubernetes resources', group: 'operations' },
  { view: 'compose', path: '/compose', label: 'Compose Import', breadcrumb: 'Compose Import', subtitle: 'Import Docker Compose into Aether workloads', group: 'operations' },
  { view: 'editor', path: '/editor', label: 'Visual Editor', breadcrumb: 'Visual Editor', subtitle: 'Form-based workload designer (no YAML required)', group: 'operations' },
  { view: 'scheduler', path: '/scheduler', label: 'Scheduler', breadcrumb: 'Scheduler', subtitle: 'Scheduling recommendations and placement', group: 'operations' },
  { view: 'health', path: '/health', label: 'Health Monitor', breadcrumb: 'Health Monitor', subtitle: 'Workload health checks and status', group: 'operations' },
  { view: 'events', path: '/events', label: 'Events', breadcrumb: 'Events', subtitle: 'Platform and workload events', group: 'operations' },
  { view: 'alerts', path: '/alerts', label: 'Alerts', breadcrumb: 'Alerts & Webhooks', subtitle: 'Notification channels, alert rules, and webhook tests', group: 'operations' },
  { view: 'platform', path: '/platform', label: 'Platform', breadcrumb: 'Platform & HA', subtitle: 'HA mode, setup recommendations, OPA, and observability', group: 'operations' },
  { view: 'sla', path: '/sla', label: 'SLA Compliance', breadcrumb: 'SLA Compliance', subtitle: 'SLA tracking and compliance', group: 'operations' },
  { view: 'deps', path: '/deps', label: 'Dependencies', breadcrumb: 'Dependencies', subtitle: 'Workload dependency graph', group: 'operations' },
  { view: 'envs', path: '/envs', label: 'Environments', breadcrumb: 'Environments', subtitle: 'Environment tiers and configuration', group: 'operations' },
  { view: 'secrets', path: '/secrets', label: 'Secrets', breadcrumb: 'Secrets', subtitle: 'Encrypted secrets management', group: 'resources' },
  { view: 'backups', path: '/backups', label: 'Backups', breadcrumb: 'Backups', subtitle: 'Backup snapshots and restore', group: 'resources' },
  { view: 'templates', path: '/templates', label: 'Templates', breadcrumb: 'Templates', subtitle: 'Workload templates library', group: 'resources' },
  { view: 'plugins', path: '/plugins', label: 'Plugins', breadcrumb: 'Plugins', subtitle: 'Runtime plugins and extensions', group: 'resources' },
  { view: 'rbac', path: '/rbac', label: 'Access Control', breadcrumb: 'Access Control', subtitle: 'API keys and role-based access', group: 'resources' },
  { view: 'audit', path: '/audit', label: 'Audit Trail', breadcrumb: 'Audit Trail', subtitle: 'Tamper-evident audit log', group: 'resources' },
  { view: 'gitops', path: '/gitops', label: 'GitOps', breadcrumb: 'GitOps', subtitle: 'GitOps reconciliation status', group: 'resources' },
  { view: 'metrics', path: '/metrics', label: 'Metrics', breadcrumb: 'Metrics', subtitle: 'Platform and workload metrics', group: 'resources' },
];

const VIEW_MAP = new Map<AppView, DashboardViewMeta>(
  DASHBOARD_VIEWS.map((v) => [v.view, v]),
);

export function getViewMeta(view: AppView): DashboardViewMeta {
  const meta = VIEW_MAP.get(view);
  if (!meta) throw new Error(`Unknown view: ${view}`);
  return meta;
}

export function getViewsByGroup(group: NavGroup): DashboardViewMeta[] {
  return DASHBOARD_VIEWS.filter((v) => v.group === group);
}

export const VIEW_TO_PATH: Record<AppView, string> = Object.fromEntries(
  DASHBOARD_VIEWS.map((v) => [v.view, v.path]),
) as Record<AppView, string>;

export const VIEW_LABELS: Record<AppView, string> = Object.fromEntries(
  DASHBOARD_VIEWS.map((v) => [v.view, v.breadcrumb]),
) as Record<AppView, string>;

export const HERO_CONFIG: Record<AppView, { title: string; subtitle: string }> = Object.fromEntries(
  DASHBOARD_VIEWS.map((v) => [v.view, { title: v.label, subtitle: v.subtitle }]),
) as Record<AppView, { title: string; subtitle: string }>;

// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
  { view: 'overview', path: '/', label: 'Overview', breadcrumb: 'Command Center', subtitle: 'Fleet briefing, savings, and capacity intelligence', group: 'primary', paletteLabel: 'Command Center' },
  { view: 'fabric', path: '/fabric', label: 'Runtime Fabric', breadcrumb: 'Runtime Fabric', subtitle: 'Live Application → Runtime → Cluster → Node topology', group: 'primary', paletteLabel: 'Fabric' },
  { view: 'migrations', path: '/migrations', label: 'Migrations', breadcrumb: 'Migrations', subtitle: 'AI migration planner with risk analysis and strategy', group: 'intelligence', paletteLabel: 'Migrations' },
  { view: 'observability', path: '/observability', label: 'Observability', breadcrumb: 'Observability', subtitle: 'Health, metrics, events, and correlated diagnostics', group: 'operations', paletteLabel: 'Observability' },
  { view: 'labs', path: '/labs', label: 'Labs', breadcrumb: 'Labs', subtitle: 'Experimental AI-generated infrastructure artifacts', group: 'resources', paletteLabel: 'Labs' },
  { view: 'settings', path: '/settings', label: 'Settings', breadcrumb: 'Settings', subtitle: 'Platform, environments, secrets, and extensions', group: 'resources', paletteLabel: 'Settings' },
  { view: 'applications', path: '/applications', label: 'Applications', breadcrumb: 'Applications', subtitle: 'Manage Kubernetes apps like an operating system — not like YAML', group: 'primary', paletteLabel: 'Applications' },
  { view: 'workloads', path: '/workloads', label: 'Workloads', breadcrumb: 'Workloads', subtitle: 'Deploy, monitor, and manage across Podman, Kubernetes, KubeVirt & Metal3', group: 'primary', paletteLabel: 'Workloads' },
  { view: 'ai', path: '/ai', label: 'AI Engine', breadcrumb: 'AI Engine', subtitle: 'Intent scoring, runtime recommendations & migration planning', group: 'intelligence', paletteLabel: 'AI Engine' },
  { view: 'zeus', path: '/zeus', label: 'Zeus', breadcrumb: 'Zeus', subtitle: 'AI infrastructure operating layer — multi-LLM, multi-agent intelligence', group: 'intelligence', paletteLabel: 'Zeus' },
  { view: 'ai-providers', path: '/settings/ai-providers', label: 'AI Providers', breadcrumb: 'AI Providers', subtitle: 'Configure OpenAI, Claude, Gemini, Grok, Ollama, and custom LLM endpoints', group: 'resources', paletteLabel: 'AI Providers' },
  { view: 'cost', path: '/cost', label: 'Cost Estimation', breadcrumb: 'Cost Estimation', subtitle: 'Resource cost projections across runtimes', group: 'intelligence', paletteLabel: 'FinOps' },
  { view: 'affinity', path: '/affinity', label: 'Runtime Affinity', breadcrumb: 'Runtime Affinity', subtitle: 'Workload class affinity and runtime fit', group: 'intelligence', paletteLabel: 'Affinity' },
  { view: 'drift', path: '/drift', label: 'Drift Detection', breadcrumb: 'Drift Detection', subtitle: 'Configuration drift & desired-state reconciliation', group: 'intelligence', paletteLabel: 'Drift' },
  { view: 'intelligence', path: '/intelligence', label: 'Intelligence Layer', breadcrumb: 'Intelligence Layer', subtitle: 'Failure predictions, threats, cost optimization, and global placement', group: 'intelligence', paletteLabel: 'Intelligence' },
  { view: 'policy', path: '/policy', label: 'Policy Check', breadcrumb: 'Policy Check', subtitle: 'Validate workloads against policy rules', group: 'intelligence', paletteLabel: 'Policy' },
  { view: 'confidential', path: '/confidential', label: 'Confidential Computing', breadcrumb: 'Confidential Computing', subtitle: 'TEE capabilities, attestation trust scores, and Ragnarok integration', group: 'intelligence', paletteLabel: 'Confidential' },
  { view: 'clusters', path: '/clusters', label: 'Cluster Browser', breadcrumb: 'Cluster Browser', subtitle: 'Browse and manage Kubernetes resources', group: 'operations', paletteLabel: 'Clusters' },
  { view: 'fleet', path: '/fleet', label: 'Fleet Overview', breadcrumb: 'Fleet Overview', subtitle: 'Multi-cluster inventory, Hubble, and PacketWolf links', group: 'operations', paletteLabel: 'Fleet' },
  { view: 'hosted', path: '/hosted', label: 'Hosted SaaS', breadcrumb: 'Hosted SaaS', subtitle: 'Tenants, API keys, metering, and Stripe billing', group: 'resources', paletteLabel: 'Hosted' },
  { view: 'activity', path: '/activity', label: 'Activity Monitor', breadcrumb: 'Activity Monitor', subtitle: 'CPU, memory, restarts, and errors across the fleet', group: 'operations', paletteLabel: 'Activity' },
  { view: 'helm', path: '/helm', label: 'Helm App Store', breadcrumb: 'Helm App Store', subtitle: 'Install curated charts with a guided wizard', group: 'operations', paletteLabel: 'Helm Store' },
  { view: 'security', path: '/security', label: 'Security Center', breadcrumb: 'Security Center', subtitle: 'Threats, secrets, policies, and hardening', group: 'intelligence', paletteLabel: 'Security' },
  { view: 'compose', path: '/compose', label: 'Compose Import', breadcrumb: 'Compose Import', subtitle: 'Import Docker Compose into Aether workloads', group: 'operations', paletteLabel: 'Docker Compose' },
  { view: 'editor', path: '/editor', label: 'Visual Editor', breadcrumb: 'Visual Editor', subtitle: 'Form-based workload designer (no YAML required)', group: 'operations', paletteLabel: 'Visual Editor' },
  { view: 'scheduler', path: '/scheduler', label: 'Scheduler', breadcrumb: 'Scheduler', subtitle: 'Scheduling recommendations and placement', group: 'operations', paletteLabel: 'Scheduler' },
  { view: 'health', path: '/health-monitor', label: 'Health Monitor', breadcrumb: 'Health Monitor', subtitle: 'Workload health checks and status', group: 'operations', paletteLabel: 'Health Monitor' },
  { view: 'events', path: '/events', label: 'Events', breadcrumb: 'Events', subtitle: 'Platform and workload events', group: 'operations', paletteLabel: 'Events' },
  { view: 'alerts', path: '/alerts', label: 'Alerts', breadcrumb: 'Alerts & Webhooks', subtitle: 'Notification channels, alert rules, and webhook tests', group: 'operations', paletteLabel: 'Alerts' },
  { view: 'platform', path: '/platform', label: 'Platform', breadcrumb: 'Platform & HA', subtitle: 'HA mode, setup recommendations, OPA, and observability', group: 'operations', paletteLabel: 'Platform' },
  { view: 'sla', path: '/sla', label: 'SLA Compliance', breadcrumb: 'SLA Compliance', subtitle: 'SLA tracking and compliance', group: 'operations', paletteLabel: 'SLA' },
  { view: 'deps', path: '/deps', label: 'Dependencies', breadcrumb: 'Dependencies', subtitle: 'Workload dependency graph', group: 'operations', paletteLabel: 'Dependencies' },
  { view: 'envs', path: '/envs', label: 'Environments', breadcrumb: 'Environments', subtitle: 'Environment tiers and configuration', group: 'operations', paletteLabel: 'Environments' },
  { view: 'secrets', path: '/secrets', label: 'Secrets', breadcrumb: 'Secrets', subtitle: 'Encrypted secrets management', group: 'resources', paletteLabel: 'Secrets' },
  { view: 'backups', path: '/backups', label: 'Backups', breadcrumb: 'Backups', subtitle: 'Backup snapshots and restore', group: 'resources', paletteLabel: 'Backups' },
  { view: 'templates', path: '/templates', label: 'Templates', breadcrumb: 'Templates', subtitle: 'Workload templates library', group: 'resources', paletteLabel: 'Templates' },
  { view: 'plugins', path: '/plugins', label: 'Plugins', breadcrumb: 'Plugins', subtitle: 'Runtime plugins and extensions', group: 'resources', paletteLabel: 'Plugins' },
  { view: 'rbac', path: '/rbac', label: 'Access Control', breadcrumb: 'Access Control', subtitle: 'API keys and role-based access', group: 'resources', paletteLabel: 'RBAC' },
  { view: 'audit', path: '/audit', label: 'Audit Trail', breadcrumb: 'Audit Trail', subtitle: 'Tamper-evident audit log', group: 'resources', paletteLabel: 'Audit' },
  { view: 'license', path: '/license', label: 'License', breadcrumb: 'License', subtitle: 'Zeus OS license status and node utilization', group: 'resources', paletteLabel: 'License' },
  { view: 'gitops', path: '/gitops', label: 'GitOps', breadcrumb: 'GitOps', subtitle: 'GitOps reconciliation status', group: 'resources', paletteLabel: 'GitOps' },
  { view: 'metrics', path: '/metrics', label: 'Metrics', breadcrumb: 'Metrics', subtitle: 'Platform and workload metrics', group: 'resources', paletteLabel: 'Metrics' },
  { view: 'openapi', path: '/openapi', label: 'API Explorer', breadcrumb: 'API Explorer', subtitle: 'Browse OpenAPI routes and raw schema', group: 'resources', paletteLabel: 'OpenAPI' },
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

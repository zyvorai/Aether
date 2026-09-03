// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { LucideIcon } from 'lucide-react';
import {
  LayoutDashboard,
  Boxes,
  Layers,
  Network,
  ArrowRightLeft,
  Eye,
  FlaskConical,
  Settings,
  Bot,
  Sparkles,
  KeyRound,
  DollarSign,
  Link2,
  GitCompareArrows,
  ShieldCheck,
  ShieldAlert,
  LockKeyhole,
  Building2,
  Server,
  Globe,
  Activity,
  Package,
  FileCode2,
  Wrench,
  CalendarClock,
  HeartPulse,
  Bell,
  BellRing,
  Radio,
  Gauge,
  GitBranch,
  Timer,
  FolderKanban,
  KeySquare,
  Archive,
  HardDrive,
  Cpu,
  FileText,
  Puzzle,
  UserCog,
  ScrollText,
  BarChart3,
  Braces,
} from 'lucide-react';
import type { AppView } from '../types/api';

export type NavGroup = 'primary' | 'intelligence' | 'operations' | 'resources';

export interface DashboardViewMeta {
  view: AppView;
  path: string;
  label: string;
  breadcrumb: string;
  subtitle: string;
  group: NavGroup;
  icon: LucideIcon;
  paletteLabel?: string;
}

export const DASHBOARD_VIEWS: DashboardViewMeta[] = [
  { view: 'overview', path: '/', label: 'Overview', breadcrumb: 'Command Center', subtitle: 'Fleet briefing, savings, and capacity intelligence', group: 'primary', icon: LayoutDashboard, paletteLabel: 'Command Center' },
  { view: 'fabric', path: '/fabric', label: 'Runtime Fabric', breadcrumb: 'Runtime Fabric', subtitle: 'Live Application → Runtime → Cluster → Node topology', group: 'primary', icon: Network, paletteLabel: 'Fabric' },
  { view: 'migrations', path: '/migrations', label: 'Migrations', breadcrumb: 'Migrations', subtitle: 'AI migration planner with risk analysis and strategy', group: 'intelligence', icon: ArrowRightLeft, paletteLabel: 'Migrations' },
  { view: 'observability', path: '/observability', label: 'Observability', breadcrumb: 'Observability', subtitle: 'Health, metrics, events, and correlated diagnostics', group: 'operations', icon: Eye, paletteLabel: 'Observability' },
  { view: 'labs', path: '/labs', label: 'Labs', breadcrumb: 'Labs', subtitle: 'Experimental AI-generated infrastructure artifacts', group: 'resources', icon: FlaskConical, paletteLabel: 'Labs' },
  { view: 'settings', path: '/settings', label: 'Settings', breadcrumb: 'Settings', subtitle: 'Platform, environments, secrets, and extensions', group: 'resources', icon: Settings, paletteLabel: 'Settings' },
  { view: 'applications', path: '/applications', label: 'Applications', breadcrumb: 'Applications', subtitle: 'Manage Kubernetes apps like an operating system — not like YAML', group: 'primary', icon: Boxes, paletteLabel: 'Applications' },
  { view: 'workloads', path: '/workloads', label: 'Workloads', breadcrumb: 'Workloads', subtitle: 'Deploy, monitor, and manage across Podman, Kubernetes, KubeVirt & Metal3', group: 'primary', icon: Layers, paletteLabel: 'Workloads' },
  { view: 'ai', path: '/ai', label: 'AI Engine', breadcrumb: 'AI Engine', subtitle: 'Intent scoring, runtime recommendations & migration planning', group: 'intelligence', icon: Bot, paletteLabel: 'AI Engine' },
  { view: 'zyra', path: '/zyra', label: 'Zyra', breadcrumb: 'Zyra', subtitle: 'AI infrastructure operating layer — multi-LLM, multi-agent intelligence', group: 'intelligence', icon: Sparkles, paletteLabel: 'Zyra' },
  { view: 'ai-providers', path: '/settings/ai-providers', label: 'AI Providers', breadcrumb: 'AI Providers', subtitle: 'Configure OpenAI, Claude, Gemini, Grok, Ollama, and custom LLM endpoints', group: 'resources', icon: KeyRound, paletteLabel: 'AI Providers' },
  { view: 'cost', path: '/cost', label: 'Cost Estimation', breadcrumb: 'Cost Estimation', subtitle: 'Resource cost projections across runtimes', group: 'intelligence', icon: DollarSign, paletteLabel: 'FinOps' },
  { view: 'affinity', path: '/affinity', label: 'Runtime Affinity', breadcrumb: 'Runtime Affinity', subtitle: 'Workload class affinity and runtime fit', group: 'intelligence', icon: Link2, paletteLabel: 'Affinity' },
  { view: 'drift', path: '/drift', label: 'Drift Detection', breadcrumb: 'Drift Detection', subtitle: 'Configuration drift & desired-state reconciliation', group: 'intelligence', icon: GitCompareArrows, paletteLabel: 'Drift' },
  { view: 'intelligence', path: '/intelligence', label: 'Intelligence Layer', breadcrumb: 'Intelligence Layer', subtitle: 'Failure predictions, threats, cost optimization, and global placement', group: 'intelligence', icon: Sparkles, paletteLabel: 'Intelligence' },
  { view: 'policy', path: '/policy', label: 'Policy Check', breadcrumb: 'Policy Check', subtitle: 'Validate workloads against policy rules', group: 'intelligence', icon: ShieldCheck, paletteLabel: 'Policy' },
  { view: 'confidential', path: '/confidential', label: 'Confidential Computing', breadcrumb: 'Confidential Computing', subtitle: 'TEE capabilities, attestation trust scores, and Ragnarok integration', group: 'intelligence', icon: LockKeyhole, paletteLabel: 'Confidential' },
  { view: 'clusters', path: '/clusters', label: 'Cluster Browser', breadcrumb: 'Cluster Browser', subtitle: 'Browse and manage Kubernetes resources', group: 'operations', icon: Server, paletteLabel: 'Clusters' },
  { view: 'fleet', path: '/fleet', label: 'Fleet Overview', breadcrumb: 'Fleet Overview', subtitle: 'Multi-cluster inventory, Hubble, and PacketWolf links', group: 'operations', icon: Globe, paletteLabel: 'Fleet' },
  { view: 'hosted', path: '/hosted', label: 'Hosted SaaS', breadcrumb: 'Hosted SaaS', subtitle: 'Tenants, API keys, metering, and Stripe billing', group: 'resources', icon: Building2, paletteLabel: 'Hosted' },
  { view: 'activity', path: '/activity', label: 'Activity Monitor', breadcrumb: 'Activity Monitor', subtitle: 'CPU, memory, restarts, and errors across the fleet', group: 'operations', icon: Activity, paletteLabel: 'Activity' },
  { view: 'helm', path: '/helm', label: 'Helm App Store', breadcrumb: 'Helm App Store', subtitle: 'Install curated charts with a guided wizard', group: 'operations', icon: Package, paletteLabel: 'Helm Store' },
  { view: 'security', path: '/security', label: 'Security Center', breadcrumb: 'Security Center', subtitle: 'Threats, secrets, policies, and hardening', group: 'intelligence', icon: ShieldAlert, paletteLabel: 'Security' },
  { view: 'compose', path: '/compose', label: 'Compose Import', breadcrumb: 'Compose Import', subtitle: 'Import Docker Compose into Aether workloads', group: 'operations', icon: FileCode2, paletteLabel: 'Docker Compose' },
  { view: 'editor', path: '/editor', label: 'Visual Editor', breadcrumb: 'Visual Editor', subtitle: 'Form-based workload designer (no YAML required)', group: 'operations', icon: Wrench, paletteLabel: 'Visual Editor' },
  { view: 'scheduler', path: '/scheduler', label: 'Scheduler', breadcrumb: 'Scheduler', subtitle: 'Scheduling recommendations and placement', group: 'operations', icon: CalendarClock, paletteLabel: 'Scheduler' },
  { view: 'health', path: '/health-monitor', label: 'Health Monitor', breadcrumb: 'Health Monitor', subtitle: 'Workload health checks and status', group: 'operations', icon: HeartPulse, paletteLabel: 'Health Monitor' },
  { view: 'events', path: '/events', label: 'Events', breadcrumb: 'Events', subtitle: 'Platform and workload events', group: 'operations', icon: Bell, paletteLabel: 'Events' },
  { view: 'alerts', path: '/alerts', label: 'Alerts', breadcrumb: 'Alerts & Webhooks', subtitle: 'Notification channels, alert rules, and webhook tests', group: 'operations', icon: BellRing, paletteLabel: 'Alerts' },
  { view: 'platform', path: '/platform', label: 'Platform', breadcrumb: 'Platform & HA', subtitle: 'HA mode, setup recommendations, OPA, and observability', group: 'operations', icon: Radio, paletteLabel: 'Platform' },
  { view: 'sla', path: '/sla', label: 'SLA Compliance', breadcrumb: 'SLA Compliance', subtitle: 'SLA tracking and compliance', group: 'operations', icon: Gauge, paletteLabel: 'SLA' },
  { view: 'deps', path: '/deps', label: 'Dependencies', breadcrumb: 'Dependencies', subtitle: 'Workload dependency graph', group: 'operations', icon: GitBranch, paletteLabel: 'Dependencies' },
  { view: 'envs', path: '/envs', label: 'Environments', breadcrumb: 'Environments', subtitle: 'Environment tiers and configuration', group: 'operations', icon: FolderKanban, paletteLabel: 'Environments' },
  { view: 'secrets', path: '/secrets', label: 'Secrets', breadcrumb: 'Secrets', subtitle: 'Encrypted secrets management', group: 'resources', icon: KeySquare, paletteLabel: 'Secrets' },
  { view: 'backups', path: '/backups', label: 'Backups', breadcrumb: 'Backups', subtitle: 'Backup snapshots and restore', group: 'resources', icon: Archive, paletteLabel: 'Backups' },
  { view: 'storage', path: '/storage', label: 'Storage', breadcrumb: 'Storage', subtitle: 'Atlas-backed persistent volumes and capacity', group: 'resources', icon: HardDrive, paletteLabel: 'Storage' },
  { view: 'forge', path: '/forge', label: 'GPU / Forge', breadcrumb: 'GPU / Forge', subtitle: 'Forge GPU capacity, nodes, and AI placement', group: 'resources', icon: Cpu, paletteLabel: 'GPU / Forge' },
  { view: 'templates', path: '/templates', label: 'Templates', breadcrumb: 'Templates', subtitle: 'Workload templates library', group: 'resources', icon: FileText, paletteLabel: 'Templates' },
  { view: 'plugins', path: '/plugins', label: 'Plugins', breadcrumb: 'Plugins', subtitle: 'Runtime plugins and extensions', group: 'resources', icon: Puzzle, paletteLabel: 'Plugins' },
  { view: 'rbac', path: '/rbac', label: 'Access Control', breadcrumb: 'Access Control', subtitle: 'API keys and role-based access', group: 'resources', icon: UserCog, paletteLabel: 'RBAC' },
  { view: 'audit', path: '/audit', label: 'Audit Trail', breadcrumb: 'Audit Trail', subtitle: 'Tamper-evident audit log', group: 'resources', icon: ScrollText, paletteLabel: 'Audit' },
  { view: 'gitops', path: '/gitops', label: 'GitOps', breadcrumb: 'GitOps', subtitle: 'GitOps reconciliation status', group: 'resources', icon: GitBranch, paletteLabel: 'GitOps' },
  { view: 'metrics', path: '/metrics', label: 'Metrics', breadcrumb: 'Metrics', subtitle: 'Platform and workload metrics', group: 'resources', icon: BarChart3, paletteLabel: 'Metrics' },
  { view: 'openapi', path: '/openapi', label: 'API Explorer', breadcrumb: 'API Explorer', subtitle: 'Browse OpenAPI routes and raw schema', group: 'resources', icon: Braces, paletteLabel: 'OpenAPI' },
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

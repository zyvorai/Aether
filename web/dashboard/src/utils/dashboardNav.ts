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
  Lock,
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
  Search,
  TrendingUp,
  MapPin,
  Rocket,
  Wand2,
  Zap,
  Target,
  Brain,
  LineChart,
  Grid3X3,
  Send,
  Webhook,
  FileCheck,
  Database,
  ExternalLink,
  RefreshCw,
  AlertTriangle,
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
  /** When false, omit from sidebar (hub child pages). Default true. */
  sidebar?: boolean;
}

export const DASHBOARD_VIEWS: DashboardViewMeta[] = [
  { view: 'overview', path: '/', label: 'Overview', breadcrumb: 'Command Center', subtitle: 'Fleet briefing, savings, and capacity intelligence', group: 'primary', icon: LayoutDashboard, paletteLabel: 'Command Center' },
  { view: 'fabric', path: '/fabric', label: 'Runtime Fabric', breadcrumb: 'Runtime Fabric', subtitle: 'Live Application → Runtime → Cluster → Node topology', group: 'primary', icon: Network, paletteLabel: 'Fabric' },
  { view: 'migrations', path: '/migrations', label: 'Migrations', breadcrumb: 'Migrations', subtitle: 'AI migration planner with risk analysis and strategy', group: 'intelligence', icon: ArrowRightLeft, paletteLabel: 'Migrations' },
  { view: 'observability', path: '/observability', label: 'Observability', breadcrumb: 'Observability', subtitle: 'Health, metrics, events, and correlated diagnostics', group: 'operations', icon: Eye, paletteLabel: 'Observability' },
  { view: 'labs', path: '/labs', label: 'Labs', breadcrumb: 'Labs', subtitle: 'Experimental AI-generated infrastructure artifacts', group: 'resources', icon: FlaskConical, paletteLabel: 'Labs' },
  { view: 'settings', path: '/settings', label: 'Settings', breadcrumb: 'Settings', subtitle: 'Platform, environments, secrets, and extensions', group: 'resources', icon: Settings, paletteLabel: 'Settings' },
  { view: 'applications', path: '/applications', label: 'Applications', breadcrumb: 'Applications', subtitle: 'Manage Kubernetes apps like an operating system — not like YAML', group: 'primary', icon: Boxes, paletteLabel: 'Applications', sidebar: false },
  { view: 'workloads', path: '/workloads', label: 'Workloads', breadcrumb: 'Workloads', subtitle: 'Deploy, monitor, and manage across Podman, Kubernetes, KubeVirt & Metal3', group: 'primary', icon: Layers, paletteLabel: 'Workloads' },
  { view: 'ai', path: '/ai', label: 'AI Engine', breadcrumb: 'AI Engine', subtitle: 'Intent scoring, runtime recommendations & migration planning', group: 'intelligence', icon: Bot, paletteLabel: 'AI Engine', sidebar: false },
  { view: 'zyra', path: '/zyra', label: 'Zyra', breadcrumb: 'Zyra', subtitle: 'AI infrastructure operating layer — multi-LLM, multi-agent intelligence', group: 'intelligence', icon: Sparkles, paletteLabel: 'Zyra', sidebar: false },
  { view: 'ai-providers', path: '/settings/ai-providers', label: 'AI Providers', breadcrumb: 'AI Providers', subtitle: 'Configure OpenAI, Claude, Gemini, Grok, Ollama, and custom LLM endpoints', group: 'resources', icon: KeyRound, paletteLabel: 'AI Providers', sidebar: false },
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

  // Observability children (hub: observability)
  { view: 'obs-root-cause', path: '/observability/root-cause', label: 'Root Cause', breadcrumb: 'Root Cause', subtitle: 'Fleet-wide root cause analysis', group: 'operations', icon: Search, sidebar: false },
  { view: 'obs-capacity-forecast', path: '/observability/capacity-forecast', label: 'Capacity Forecast', breadcrumb: 'Capacity Forecast', subtitle: 'Predictive capacity planning', group: 'operations', icon: TrendingUp, sidebar: false },
  { view: 'obs-capacity-scale', path: '/observability/capacity-scale', label: 'Capacity Scale', breadcrumb: 'Capacity Scale', subtitle: 'Scale recommendations and actions', group: 'operations', icon: Gauge, sidebar: false },
  { view: 'obs-self-healing', path: '/observability/self-healing', label: 'Self-Healing', breadcrumb: 'Self-Healing', subtitle: 'Automated remediation loops', group: 'operations', icon: HeartPulse, sidebar: false },
  { view: 'obs-autonomous-sre', path: '/observability/autonomous-sre', label: 'Autonomous SRE', breadcrumb: 'Autonomous SRE', subtitle: 'Autonomous SRE agent workflows', group: 'operations', icon: Bot, sidebar: false },
  { view: 'obs-extensions', path: '/observability/extensions', label: 'Observability Extensions', breadcrumb: 'Extensions', subtitle: 'Extensions graduation and rollout', group: 'operations', icon: Puzzle, sidebar: false },
  { view: 'obs-reliability', path: '/observability/reliability', label: 'Reliability', breadcrumb: 'Reliability', subtitle: 'SRE reliability signals', group: 'operations', icon: ShieldCheck, sidebar: false },

  // Settings children (hub: settings)
  { view: 'settings-identity', path: '/settings/identity', label: 'Identity & SSO', breadcrumb: 'Identity & SSO', subtitle: 'Identity providers and SSO', group: 'resources', icon: UserCog, sidebar: false },
  { view: 'settings-nav', path: '/settings/navigation', label: 'Navigation', breadcrumb: 'Navigation', subtitle: 'Sidebar and navigation preferences', group: 'resources', icon: Settings, sidebar: false },
  { view: 'settings-autonomous', path: '/settings/autonomous', label: 'Autonomous Mode', breadcrumb: 'Autonomous Mode', subtitle: 'Autonomous platform mode', group: 'resources', icon: Sparkles, sidebar: false },
  { view: 'settings-macos', path: '/settings/macos', label: 'macOS', breadcrumb: 'macOS', subtitle: 'macOS platform integration', group: 'resources', icon: Cpu, sidebar: false },

  // Labs children (hub: labs)
  { view: 'labs-live', path: '/labs/live', label: 'Live Labs', breadcrumb: 'Live Labs', subtitle: 'Experimental live lab sessions', group: 'resources', icon: FlaskConical, sidebar: false },
  { view: 'labs-graduation', path: '/labs/graduation', label: 'Labs Graduation', breadcrumb: 'Graduation', subtitle: 'Graduate lab artifacts to production', group: 'resources', icon: Rocket, sidebar: false },
  { view: 'labs-graph', path: '/labs/knowledge-graph', label: 'Knowledge Graph', breadcrumb: 'Knowledge Graph', subtitle: 'Infrastructure knowledge graph', group: 'resources', icon: GitBranch, sidebar: false },
  { view: 'labs-platform', path: '/labs/graph-platform', label: 'Graph Platform', breadcrumb: 'Graph Platform', subtitle: 'Graph platform controls', group: 'resources', icon: Network, sidebar: false },

  // Migrations children (hub: migrations)
  { view: 'mig-planner', path: '/migrations/planner', label: 'Migration Planner', breadcrumb: 'Planner', subtitle: 'AI migration planner with risk analysis', group: 'intelligence', icon: ArrowRightLeft, sidebar: false },
  { view: 'mig-placement', path: '/migrations/placement', label: 'Autonomous Placement', breadcrumb: 'Placement', subtitle: 'Autonomous placement recommendations', group: 'intelligence', icon: MapPin, sidebar: false },
  { view: 'mig-replication', path: '/migrations/replication', label: 'Volume Replication', breadcrumb: 'Replication', subtitle: 'Volume replication for migrations', group: 'intelligence', icon: HardDrive, sidebar: false },
  { view: 'mig-waves', path: '/migrations/waves', label: 'Migration Waves', breadcrumb: 'Waves', subtitle: 'Wave-based migration orchestration', group: 'intelligence', icon: Layers, sidebar: false },

  // Fabric children (hub: fabric)
  { view: 'fabric-twin', path: '/fabric/digital-twin', label: 'Digital Twin', breadcrumb: 'Digital Twin', subtitle: 'Digital twin of the runtime fabric', group: 'primary', icon: Sparkles, sidebar: false },
  { view: 'fabric-topology', path: '/fabric/topology', label: 'Fabric Topology', breadcrumb: 'Topology', subtitle: 'Application → Runtime → Cluster → Node topology', group: 'primary', icon: Network, sidebar: false },
  { view: 'fabric-graph', path: '/fabric/knowledge-graph', label: 'Fabric Knowledge Graph', breadcrumb: 'Knowledge Graph', subtitle: 'Fabric knowledge graph', group: 'primary', icon: GitBranch, sidebar: false },
  { view: 'fabric-unified', path: '/fabric/unified', label: 'Unified Fabric', breadcrumb: 'Unified Fabric', subtitle: 'Unified fabric view', group: 'primary', icon: Globe, sidebar: false },

  // Hub children (AI / Intelligence / Fleet / Confidential)
  { view: 'ai-advisor', path: '/ai/advisor', label: 'Runtime Advisor', breadcrumb: 'Runtime Advisor', subtitle: 'Compare runtimes with confidence scores', group: 'intelligence', icon: Brain, sidebar: false },
  { view: 'ai-designer', path: '/ai/designer', label: 'Workload Designer', breadcrumb: 'Workload Designer', subtitle: 'Design workloads from intent', group: 'intelligence', icon: Wand2, sidebar: false },
  { view: 'ai-intent', path: '/ai/intent', label: 'Intent Studio', breadcrumb: 'Intent Studio', subtitle: 'Shape and score workload intent', group: 'intelligence', icon: Sparkles, sidebar: false },
  { view: 'ai-pipeline', path: '/ai/pipeline', label: 'Intent Pipeline', breadcrumb: 'Pipeline', subtitle: 'Intent pipeline and stages', group: 'intelligence', icon: Wand2, sidebar: false },
  { view: 'ai-recommend', path: '/ai/recommend', label: 'Advanced Scoring', breadcrumb: 'Advanced', subtitle: 'Deep recommendation workspace', group: 'intelligence', icon: Zap, sidebar: false },
  { view: 'ai-optimize', path: '/ai/optimize', label: 'Optimization', breadcrumb: 'Optimization', subtitle: 'Resize and efficiency advice', group: 'intelligence', icon: Target, sidebar: false },
  { view: 'ai-analyze', path: '/ai/analyze', label: 'Analysis', breadcrumb: 'Analysis', subtitle: 'Profile and log analysis', group: 'intelligence', icon: Cpu, sidebar: false },
  { view: 'intel-predictions', path: '/intelligence/predictions', label: 'Predictions', breadcrumb: 'Predictions', subtitle: 'Fleet failure predictions', group: 'intelligence', icon: TrendingUp, sidebar: false },
  { view: 'intel-threats', path: '/intelligence/threats', label: 'Threats', breadcrumb: 'Threats', subtitle: 'Threat intelligence', group: 'intelligence', icon: ShieldAlert, sidebar: false },
  { view: 'intel-cost', path: '/intelligence/cost', label: 'Cost Optimize', breadcrumb: 'Cost Optimize', subtitle: 'Cost optimization recommendations', group: 'intelligence', icon: DollarSign, sidebar: false },
  { view: 'intel-evolution', path: '/intelligence/evolution', label: 'Evolution', breadcrumb: 'Evolution', subtitle: 'Runtime evolution status', group: 'intelligence', icon: Sparkles, sidebar: false },
  { view: 'intel-placement', path: '/intelligence/placement', label: 'Placement', breadcrumb: 'Placement', subtitle: 'Global placement recommendations', group: 'intelligence', icon: MapPin, sidebar: false },
  { view: 'fleet-overview', path: '/fleet/overview', label: 'Fleet Overview', breadcrumb: 'Overview', subtitle: 'Clusters and network observability', group: 'operations', icon: Globe, sidebar: false },
  { view: 'fleet-edge', path: '/fleet/edge', label: 'Edge Fleet', breadcrumb: 'Edge', subtitle: 'Edge fleet inventory', group: 'operations', icon: Network, sidebar: false },
  { view: 'fleet-placement', path: '/fleet/placement', label: 'Fleet Placement', breadcrumb: 'Placement', subtitle: 'Federation placement planning', group: 'operations', icon: MapPin, sidebar: false },
  { view: 'conf-tee', path: '/confidential/tee', label: 'TEE & Integration', breadcrumb: 'TEE', subtitle: 'Confidential computing integration and host TEE', group: 'intelligence', icon: LockKeyhole, sidebar: false },
  { view: 'conf-trust', path: '/confidential/trust', label: 'Trust Scores', breadcrumb: 'Trust', subtitle: 'Fleet attestation trust scores', group: 'intelligence', icon: ShieldCheck, sidebar: false },
  { view: 'conf-migrate', path: '/confidential/migrate', label: 'Encrypted Migration', breadcrumb: 'Migration', subtitle: 'Confidential migration wizard', group: 'intelligence', icon: ArrowRightLeft, sidebar: false },
  { view: 'conf-workloads', path: '/confidential/workloads', label: 'Confidential Workloads', breadcrumb: 'Workloads', subtitle: 'Confidential workload inventory', group: 'intelligence', icon: Layers, sidebar: false },

  // Dense-page children
  { view: 'activity-cpu', path: '/activity/cpu', label: 'CPU', breadcrumb: 'CPU', subtitle: 'Top CPU pods', group: 'operations', icon: Cpu, sidebar: false },
  { view: 'activity-memory', path: '/activity/memory', label: 'Memory', breadcrumb: 'Memory', subtitle: 'Top memory consumers', group: 'operations', icon: HardDrive, sidebar: false },
  { view: 'activity-restarts', path: '/activity/restarts', label: 'Restarts', breadcrumb: 'Restarts', subtitle: 'Frequent restarts', group: 'operations', icon: RefreshCw, sidebar: false },
  { view: 'activity-errors', path: '/activity/errors', label: 'Errors', breadcrumb: 'Errors', subtitle: 'Recent error events', group: 'operations', icon: AlertTriangle, sidebar: false },
  { view: 'affinity-recommend', path: '/affinity/recommend', label: 'Recommendations', breadcrumb: 'Recommendations', subtitle: 'Runtime affinity recommendations', group: 'intelligence', icon: Link2, sidebar: false },
  { view: 'affinity-matrix', path: '/affinity/matrix', label: 'Matrix', breadcrumb: 'Matrix', subtitle: 'Compatibility matrix', group: 'intelligence', icon: Grid3X3, sidebar: false },
  { view: 'affinity-stats', path: '/affinity/stats', label: 'Stats', breadcrumb: 'Stats', subtitle: 'Affinity statistics', group: 'intelligence', icon: BarChart3, sidebar: false },
  { view: 'clusters-browse', path: '/clusters/browse', label: 'Browse', breadcrumb: 'Browse', subtitle: 'Browse Kubernetes resources', group: 'operations', icon: Server, sidebar: false },
  { view: 'clusters-network', path: '/clusters/network', label: 'Network', breadcrumb: 'Network', subtitle: 'Network and Cilium policies', group: 'operations', icon: Network, sidebar: false },
  { view: 'platform-recommendations', path: '/platform/recommendations', label: 'Recommendations', breadcrumb: 'Recommendations', subtitle: 'Setup recommendations', group: 'operations', icon: Radio, sidebar: false },
  { view: 'platform-trust', path: '/platform/trust', label: 'Production trust', breadcrumb: 'Trust', subtitle: 'Production trust signals', group: 'operations', icon: ShieldCheck, sidebar: false },
  { view: 'platform-ecosystem', path: '/platform/ecosystem', label: 'Ecosystem', breadcrumb: 'Ecosystem', subtitle: 'Ecosystem integrations', group: 'operations', icon: Network, sidebar: false },
  { view: 'platform-runtime', path: '/platform/runtime', label: 'Runtime & policy', breadcrumb: 'Runtime', subtitle: 'API server, HA, OPA', group: 'operations', icon: Server, sidebar: false },
  { view: 'platform-cilium', path: '/platform/cilium', label: 'Kubernetes / Cilium', breadcrumb: 'Cilium', subtitle: 'CNI status and connectivity', group: 'operations', icon: Network, sidebar: false },
  { view: 'platform-observability', path: '/platform/observability', label: 'Observability links', breadcrumb: 'Links', subtitle: 'Grafana, Prometheus, Hubble', group: 'operations', icon: ExternalLink, sidebar: false },
  { view: 'metrics-summary', path: '/metrics/summary', label: 'Platform metrics', breadcrumb: 'Summary', subtitle: 'API and cluster metrics', group: 'resources', icon: BarChart3, sidebar: false },
  { view: 'metrics-chargeback', path: '/metrics/chargeback', label: 'Chargeback', breadcrumb: 'Chargeback', subtitle: 'Fleet showback', group: 'resources', icon: DollarSign, sidebar: false },
  { view: 'metrics-observability', path: '/metrics/observability', label: 'External links', breadcrumb: 'Links', subtitle: 'Grafana and Prometheus', group: 'resources', icon: ExternalLink, sidebar: false },
  { view: 'metrics-prometheus', path: '/metrics/prometheus', label: 'Prometheus', breadcrumb: 'Prometheus', subtitle: 'Query explorer and raw metrics', group: 'resources', icon: LineChart, sidebar: false },
  { view: 'security-overview', path: '/security/overview', label: 'Overview', breadcrumb: 'Overview', subtitle: 'Risk summary', group: 'intelligence', icon: ShieldAlert, sidebar: false },
  { view: 'security-copilot', path: '/security/copilot', label: 'Security copilot', breadcrumb: 'Copilot', subtitle: 'AI security guidance', group: 'intelligence', icon: Sparkles, sidebar: false },
  { view: 'security-platform', path: '/security/platform', label: 'Security platform', breadcrumb: 'Platform', subtitle: 'Platform security controls', group: 'intelligence', icon: ShieldCheck, sidebar: false },
  { view: 'security-sbom', path: '/security/sbom', label: 'SBOM & images', breadcrumb: 'SBOM', subtitle: 'SBOM and signed images', group: 'intelligence', icon: FileCheck, sidebar: false },
  { view: 'security-remediation', path: '/security/remediation', label: 'Remediation', breadcrumb: 'Remediation', subtitle: 'Anomaly remediation', group: 'intelligence', icon: AlertTriangle, sidebar: false },
  { view: 'security-threats', path: '/security/threats', label: 'Threats & secrets', breadcrumb: 'Threats', subtitle: 'Threat scan and secrets', group: 'intelligence', icon: Lock, sidebar: false },
  { view: 'gitops-center', path: '/gitops/center', label: 'GitOps center', breadcrumb: 'Center', subtitle: 'Repo status and sync', group: 'resources', icon: GitBranch, sidebar: false },
  { view: 'gitops-agent', path: '/gitops/agent', label: 'GitOps agent', breadcrumb: 'Agent', subtitle: 'Autonomous GitOps agent', group: 'resources', icon: Bot, sidebar: false },
  { view: 'gitops-intent', path: '/gitops/intent', label: 'Intent diff', breadcrumb: 'Intent', subtitle: 'Intent vs GitOps drift', group: 'resources', icon: GitCompareArrows, sidebar: false },
  { view: 'gitops-sync', path: '/gitops/sync', label: 'Reconciliation', breadcrumb: 'Sync', subtitle: 'Init and sync tables', group: 'resources', icon: GitBranch, sidebar: false },
  { view: 'cost-intelligence', path: '/cost/intelligence', label: 'Cost intelligence', breadcrumb: 'Intelligence', subtitle: 'AI cost insights', group: 'intelligence', icon: DollarSign, sidebar: false },
  { view: 'cost-finops', path: '/cost/finops', label: 'FinOps platform', breadcrumb: 'FinOps', subtitle: 'FinOps controls', group: 'intelligence', icon: DollarSign, sidebar: false },
  { view: 'cost-estimate', path: '/cost/estimate', label: 'Estimator', breadcrumb: 'Estimator', subtitle: 'Cost projections', group: 'intelligence', icon: DollarSign, sidebar: false },
  { view: 'alerts-channels', path: '/alerts/channels', label: 'Channels', breadcrumb: 'Channels', subtitle: 'Notification channels', group: 'operations', icon: Radio, sidebar: false },
  { view: 'alerts-rules', path: '/alerts/rules', label: 'Alert rules', breadcrumb: 'Rules', subtitle: 'Configured alert rules', group: 'operations', icon: BellRing, sidebar: false },
  { view: 'alerts-test', path: '/alerts/test', label: 'Test webhook', breadcrumb: 'Test', subtitle: 'Send test webhook', group: 'operations', icon: Send, sidebar: false },
  { view: 'alerts-queue', path: '/alerts/queue', label: 'Retry queue', breadcrumb: 'Queue', subtitle: 'Webhook retry queue', group: 'operations', icon: Webhook, sidebar: false },
];

const VIEW_MAP = new Map<AppView, DashboardViewMeta>(
  DASHBOARD_VIEWS.map((v) => [v.view, v]),
);

export function getViewMeta(view: AppView): DashboardViewMeta {
  const meta = VIEW_MAP.get(view);
  if (!meta) throw new Error(`Unknown view: ${view}`);
  return meta;
}

export function getViewsByGroup(group: NavGroup, opts?: { sidebarOnly?: boolean }): DashboardViewMeta[] {
  return DASHBOARD_VIEWS.filter((v) => {
    if (v.group !== group) return false;
    if (opts?.sidebarOnly && v.sidebar === false) return false;
    return true;
  });
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

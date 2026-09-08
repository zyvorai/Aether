// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { ChevronRight, Home } from 'lucide-react';
import { useNavigate } from 'react-router';
import type { AppView } from '../types/api';
import { VIEW_LABELS, getViewMeta, type NavGroup } from '../utils/dashboardNav';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';

interface BreadcrumbProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  workloadName?: string;
}

const GROUP_LABELS: Record<NavGroup, string> = {
  primary: 'Overview',
  intelligence: 'Intelligence',
  operations: 'Operations',
  resources: 'Resources',
};

// Explicit parent overrides for hub sub-pages (takes precedence over group).
const PARENT_CRUMBS: Partial<Record<AppView, { view: AppView; label: string }>> = {
  'ai-providers': { view: 'settings', label: 'Settings' },
  'settings-identity': { view: 'settings', label: 'Settings' },
  'settings-nav': { view: 'settings', label: 'Settings' },
  'settings-autonomous': { view: 'settings', label: 'Settings' },
  'settings-macos': { view: 'settings', label: 'Settings' },
  'obs-root-cause': { view: 'observability', label: 'Observability' },
  'obs-capacity-forecast': { view: 'observability', label: 'Observability' },
  'obs-capacity-scale': { view: 'observability', label: 'Observability' },
  'obs-self-healing': { view: 'observability', label: 'Observability' },
  'obs-autonomous-sre': { view: 'observability', label: 'Observability' },
  'obs-extensions': { view: 'observability', label: 'Observability' },
  'obs-reliability': { view: 'observability', label: 'Observability' },
  'labs-live': { view: 'labs', label: 'Labs' },
  'labs-graduation': { view: 'labs', label: 'Labs' },
  'labs-graph': { view: 'labs', label: 'Labs' },
  'labs-platform': { view: 'labs', label: 'Labs' },
  'mig-planner': { view: 'migrations', label: 'Migrations' },
  'mig-placement': { view: 'migrations', label: 'Migrations' },
  'mig-replication': { view: 'migrations', label: 'Migrations' },
  'mig-waves': { view: 'migrations', label: 'Migrations' },
  'fabric-twin': { view: 'fabric', label: 'Runtime Fabric' },
  'fabric-topology': { view: 'fabric', label: 'Runtime Fabric' },
  'fabric-graph': { view: 'fabric', label: 'Runtime Fabric' },
  'fabric-unified': { view: 'fabric', label: 'Runtime Fabric' },
  'ai-advisor': { view: 'ai', label: 'AI Engine' },
  'ai-designer': { view: 'ai', label: 'AI Engine' },
  'ai-intent': { view: 'ai', label: 'AI Engine' },
  'ai-pipeline': { view: 'ai', label: 'AI Engine' },
  'ai-recommend': { view: 'ai', label: 'AI Engine' },
  'ai-optimize': { view: 'ai', label: 'AI Engine' },
  'ai-analyze': { view: 'ai', label: 'AI Engine' },
  'intel-predictions': { view: 'intelligence', label: 'Intelligence Layer' },
  'intel-threats': { view: 'intelligence', label: 'Intelligence Layer' },
  'intel-cost': { view: 'intelligence', label: 'Intelligence Layer' },
  'intel-evolution': { view: 'intelligence', label: 'Intelligence Layer' },
  'intel-placement': { view: 'intelligence', label: 'Intelligence Layer' },
  'fleet-overview': { view: 'fleet', label: 'Fleet Overview' },
  'fleet-edge': { view: 'fleet', label: 'Fleet Overview' },
  'fleet-placement': { view: 'fleet', label: 'Fleet Overview' },
  ai: { view: 'intelligence', label: 'Intelligence Layer' },
  zyra: { view: 'intelligence', label: 'Intelligence Layer' },
  copilot: { view: 'intelligence', label: 'Intelligence Layer' },
  'activity-cpu': { view: 'activity', label: 'Activity Monitor' },
  'activity-memory': { view: 'activity', label: 'Activity Monitor' },
  'activity-restarts': { view: 'activity', label: 'Activity Monitor' },
  'activity-errors': { view: 'activity', label: 'Activity Monitor' },
  'affinity-recommend': { view: 'affinity', label: 'Runtime Affinity' },
  'affinity-matrix': { view: 'affinity', label: 'Runtime Affinity' },
  'affinity-stats': { view: 'affinity', label: 'Runtime Affinity' },
  'clusters-browse': { view: 'clusters', label: 'Cluster Browser' },
  'clusters-network': { view: 'clusters', label: 'Cluster Browser' },
  'platform-recommendations': { view: 'platform', label: 'Platform' },
  'platform-trust': { view: 'platform', label: 'Platform' },
  'platform-ecosystem': { view: 'platform', label: 'Platform' },
  'platform-runtime': { view: 'platform', label: 'Platform' },
  'platform-cilium': { view: 'platform', label: 'Platform' },
  'platform-observability': { view: 'platform', label: 'Platform' },
  'metrics-summary': { view: 'metrics', label: 'Metrics' },
  'metrics-chargeback': { view: 'metrics', label: 'Metrics' },
  'metrics-observability': { view: 'metrics', label: 'Metrics' },
  'metrics-prometheus': { view: 'metrics', label: 'Metrics' },
  'security-overview': { view: 'security', label: 'Security Center' },
  'security-copilot': { view: 'security', label: 'Security Center' },
  'security-platform': { view: 'security', label: 'Security Center' },
  'security-sbom': { view: 'security', label: 'Security Center' },
  'security-remediation': { view: 'security', label: 'Security Center' },
  'security-threats': { view: 'security', label: 'Security Center' },
  'gitops-center': { view: 'gitops', label: 'GitOps' },
  'gitops-agent': { view: 'gitops', label: 'GitOps' },
  'gitops-intent': { view: 'gitops', label: 'GitOps' },
  'gitops-sync': { view: 'gitops', label: 'GitOps' },
  'cost-intelligence': { view: 'cost', label: 'Cost Estimation' },
  'cost-finops': { view: 'cost', label: 'Cost Estimation' },
  'cost-estimate': { view: 'cost', label: 'Cost Estimation' },
  'alerts-channels': { view: 'alerts', label: 'Alerts' },
  'alerts-rules': { view: 'alerts', label: 'Alerts' },
  'alerts-test': { view: 'alerts', label: 'Alerts' },
  'alerts-queue': { view: 'alerts', label: 'Alerts' },
};

export default function Breadcrumb({ currentView, onNavigate, workloadName }: BreadcrumbProps) {
  const navigate = useNavigate();
  if (currentView === 'overview') return null;

  const label = VIEW_LABELS[currentView];
  const workload = workloadName?.trim();
  const explicitParent = PARENT_CRUMBS[currentView];
  const group = (() => {
    try {
      return getViewMeta(currentView).group;
    } catch {
      return undefined;
    }
  })();
  const groupLabel = !explicitParent && group && group !== 'primary' ? GROUP_LABELS[group] : null;

  return (
    <nav className="dash-breadcrumb mb-3" aria-label="Breadcrumb">
      <div className="flex min-w-0 flex-wrap items-center gap-0.5 px-1 py-1">
        <button
          type="button"
          onClick={() => onNavigate('overview')}
          className="inline-flex items-center gap-1.5 rounded-lg px-2 py-1 text-[13px] text-muted transition hover:bg-white/[0.04] hover:text-foreground focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40"
        >
          <Home className="h-3.5 w-3.5 shrink-0 opacity-70" aria-hidden />
          <span className="hidden sm:inline">Home</span>
        </button>
        {explicitParent ? (
          <>
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-subtle" aria-hidden />
            <button
              type="button"
              data-testid="breadcrumb-parent"
              onClick={() => onNavigate(explicitParent.view)}
              className="truncate rounded-lg px-2 py-1 text-[13px] text-muted transition hover:bg-white/[0.04] hover:text-foreground"
            >
              {explicitParent.label}
            </button>
          </>
        ) : null}
        {groupLabel ? (
          <>
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-subtle" aria-hidden />
            <span data-testid="breadcrumb-group" className="truncate px-2 py-1 text-[13px] text-subtle">
              {groupLabel}
            </span>
          </>
        ) : null}
        <ChevronRight className="h-3.5 w-3.5 shrink-0 text-subtle" aria-hidden />
        <span className="truncate px-2 py-1 text-[13px] font-medium text-foreground">{label}</span>
        {workload ? (
          <>
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-subtle" aria-hidden />
            <button
              type="button"
              data-testid="breadcrumb-workload"
              onClick={() => navigate(pathWithQuery(viewToPath(currentView), { workload }))}
              className="max-w-[12rem] truncate rounded-lg px-2 py-1 text-[13px] font-mono text-primary transition hover:bg-white/[0.04] hover:underline"
            >
              {workload}
            </button>
          </>
        ) : null}
      </div>
    </nav>
  );
}

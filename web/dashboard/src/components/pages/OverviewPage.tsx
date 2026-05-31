// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router';
import {
  LayoutDashboard,
  Activity,
  AlertTriangle,
  Calendar,
  Shield,
  Lock,
  Inbox,
  Boxes,
  KeySquare,
  Workflow,
  RefreshCw,
  Rocket,
  WifiOff,
  Layers,
  Gauge,
} from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import type { AppView } from '../../types/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { hasValidatedSpec, hasDeployedWorkload, hasReviewedHealth, syncDeployFromWorkloads, syncHealthFromSummary } from '../../utils/onboardingState';
import { countAetherManaged } from '../../utils/workloadFilters';
import CommandCenterBriefing from '../CommandCenterBriefing';
import CommandCenterIntentSla from '../CommandCenterIntentSla';
import CommandCenterNextActions from '../CommandCenterNextActions';
import CommandMetricCard from '../CommandMetricCard';
import StatCard from '../StatCard';
import { SeverityBadge } from '../Badge';
import OnboardingStrip from '../OnboardingStrip';
import EmptyState from '../EmptyState';
import PlatformStatusPanel from '../PlatformStatusPanel';
import K8sControlCenter from '../K8sControlCenter';
import { isK8sApplication } from '../../utils/k8sUx';
import { useServerCapabilities } from '../../contexts/ServerCapabilitiesContext';
import { platformSetupNeeded } from '../../utils/navCapabilities';
import type {
  WorkloadResponse,
  EventSummary,
  HealthSummary,
  BackupInfo,
  SecretSummary,
  Event,
  ClusterSummary,
  PluginInfo,
  Environment,
  ApiKeySummary,
} from '../../types/api';

interface OverviewPageProps {
  username?: string;
  onNavigate: (view: AppView) => void;
  sseConnected?: boolean;
  refreshKey?: number;
}

type OverviewEndpoint =
  | 'workloads'
  | 'eventsSummary'
  | 'health'
  | 'backups'
  | 'secrets'
  | 'events'
  | 'clusters'
  | 'plugins'
  | 'environments'
  | 'apiKeys';

const ENDPOINT_LABELS: Record<OverviewEndpoint, string> = {
  workloads: 'Workloads',
  eventsSummary: 'Event summary',
  health: 'Health',
  backups: 'Backups',
  secrets: 'Secrets',
  events: 'Events',
  clusters: 'Clusters',
  plugins: 'Plugins',
  environments: 'Environments',
  apiKeys: 'API keys',
};

type QuickLink = {
  label: string;
  testId: string;
  onClick: () => void;
};

function OverviewFleetSnapshot({
  workloadsCount,
  aetherManagedCount,
  clusterDiscoveredCount,
  sseConnected,
  healthy,
  degraded,
  pluginsCount,
  environmentsCount,
  apiKeysCount,
  onNavigate,
}: {
  workloadsCount: number;
  aetherManagedCount: number;
  clusterDiscoveredCount: number;
  sseConnected: boolean;
  healthy: number;
  degraded: number;
  pluginsCount: number;
  environmentsCount: number;
  apiKeysCount: number;
  onNavigate: (view: AppView) => void;
}) {
  const fabricEmpty = workloadsCount === 0;
  const signalEmpty = healthy === 0 && degraded === 0;
  const controlEmpty = pluginsCount === 0 && environmentsCount === 0 && apiKeysCount === 0;

  return (
    <section className="overview-section-shell mb-8 p-6 sm:p-8">
      <div className="overview-section-header">
        <p className="section-label">Fleet</p>
        <h2 className="section-title">Snapshot</h2>
        <p className="section-subtitle">Runtime fabric, signal quality, and control surface at a glance</p>
      </div>
      <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
        <CommandMetricCard
          label="Runtime Fabric"
          value={workloadsCount}
          icon={<Layers className="h-3.5 w-3.5" />}
          accent={fabricEmpty ? 'muted' : 'blue'}
          hint={`${aetherManagedCount} Aether-managed · ${clusterDiscoveredCount} discovered`}
          isEmpty={fabricEmpty}
          onClick={() => onNavigate('fabric')}
          valueTestId="overview-runtime-fabric-count"
        >
          <span className="absolute right-4 top-4 rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2 py-0.5 text-[10px] text-emerald-300">
            {sseConnected ? 'live' : 'syncing'}
          </span>
        </CommandMetricCard>

        <CommandMetricCard
          label="Signal Quality"
          value={healthy}
          icon={<Gauge className="h-3.5 w-3.5" />}
          accent={signalEmpty ? 'muted' : 'emerald'}
          hint={signalEmpty ? 'Health checks activate with workloads' : `${degraded} degraded · View health →`}
          isEmpty={signalEmpty}
          onClick={() => onNavigate('health')}
        >
          {!signalEmpty ? (
            <div className="relative mt-4 h-1.5 overflow-hidden rounded-full bg-slate-800/80">
              <div
                className="h-full rounded-full bg-gradient-to-r from-emerald-400 to-aether"
                style={{ width: `${Math.min(100, Math.max(8, (healthy / Math.max(1, healthy + degraded)) * 100))}%` }}
              />
            </div>
          ) : null}
        </CommandMetricCard>

        <div className="glass-metric-card glass-metric-accent-purple">
          <div className="relative mb-3 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
            <Boxes className="h-3.5 w-3.5" />
            Control Surface
          </div>
          <div className="relative grid grid-cols-3 gap-3 text-center">
            {[
              { label: 'Plugins', value: pluginsCount },
              { label: 'Envs', value: environmentsCount },
              { label: 'Keys', value: apiKeysCount },
            ].map((item) => (
              <div
                key={item.label}
                className="rounded-xl border border-slate-800/60 bg-[#11151C] p-3"
              >
                <div className={`text-lg font-semibold ${controlEmpty ? 'text-slate-500' : 'text-white'}`}>
                  {item.value}
                </div>
                <div className="text-[11px] text-slate-500">{item.label}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

function OverviewQuickAccess({ links }: { links: QuickLink[] }) {
  return (
    <section className="overview-section-shell mb-8 p-6 sm:p-8">
      <div className="overview-section-header flex flex-wrap items-end justify-between gap-3">
        <div>
          <p className="section-label">Navigation</p>
          <h2 className="section-title">Quick access</h2>
          <p className="section-subtitle">Jump to platform tools and intelligence views</p>
        </div>
      </div>
      <div className="flex flex-wrap gap-2">
        {links.map((link) => (
          <button
            key={link.label}
            type="button"
            onClick={link.onClick}
            data-testid={link.testId}
            className="quick-link-chip"
          >
            {link.label}
          </button>
        ))}
      </div>
    </section>
  );
}

export default function OverviewPage({ username = '', onNavigate, sseConnected = false, refreshKey = 0 }: OverviewPageProps) {
  const navigate = useNavigate();
  const [workloadFocus] = useQueryParam('workload');
  const focusedWorkload = workloadFocus.trim();
  const { capabilities, ready, loading: platformLoading } = useServerCapabilities();

  const goFiltered = (view: AppView, params?: Record<string, string>) => {
    navigate(pathWithQuery(viewToPath(view), params ?? {}));
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [eventSummary, setEventSummary] = useState<EventSummary | null>(null);
  const [healthSummary, setHealthSummary] = useState<HealthSummary | null>(null);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [events, setEvents] = useState<Event[]>([]);
  const [clusterSummary, setClusterSummary] = useState<ClusterSummary | null>(null);
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [environments, setEnvironments] = useState<Environment[]>([]);
  const [apiKeys, setApiKeys] = useState<ApiKeySummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [failedEndpoints, setFailedEndpoints] = useState<OverviewEndpoint[]>([]);
  const [totalFailure, setTotalFailure] = useState(false);
  const [specValidated, setSpecValidated] = useState(() => hasValidatedSpec());
  const [deployDone, setDeployDone] = useState(() => hasDeployedWorkload());
  const [healthDone, setHealthDone] = useState(() => hasReviewedHealth());

  useEffect(() => {
    const onValidated = () => setSpecValidated(true);
    const onDeploy = () => setDeployDone(true);
    const onHealth = () => setHealthDone(true);
    window.addEventListener('aether:spec-validated', onValidated);
    window.addEventListener('aether:first-deploy', onDeploy);
    window.addEventListener('aether:health-reviewed', onHealth);
    return () => {
      window.removeEventListener('aether:spec-validated', onValidated);
      window.removeEventListener('aether:first-deploy', onDeploy);
      window.removeEventListener('aether:health-reviewed', onHealth);
    };
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setTotalFailure(false);
    setFailedEndpoints([]);

    const requests = await Promise.allSettled([
      apiFetchSettled<WorkloadResponse[]>('/workloads'),
      apiFetchSettled<EventSummary>('/events/summary'),
      apiFetchSettled<HealthSummary>('/orchestrator/summary'),
      apiFetchSettled<BackupInfo[]>('/backups'),
      apiFetchSettled<SecretSummary[]>('/secrets'),
      apiFetchSettled<Event[]>('/events'),
      apiFetchSettled<ClusterSummary>('/cluster/summary'),
      apiFetchSettled<PluginInfo[]>('/plugins'),
      apiFetchSettled<Environment[]>('/environments'),
      apiFetchSettled<ApiKeySummary[]>('/rbac/keys'),
    ]);

    const keys: OverviewEndpoint[] = [
      'workloads',
      'eventsSummary',
      'health',
      'backups',
      'secrets',
      'events',
      'clusters',
      'plugins',
      'environments',
      'apiKeys',
    ];

    const failed: OverviewEndpoint[] = [];
    let successCount = 0;

    requests.forEach((result, i) => {
      const key = keys[i];
      if (result.status === 'rejected' || !result.value.ok) {
        failed.push(key);
        return;
      }
      successCount += 1;
      const data = result.value.data;
      switch (key) {
        case 'workloads': {
          const list = data as WorkloadResponse[];
          setWorkloads(list);
          syncDeployFromWorkloads(list);
          if (countAetherManaged(list) > 0) setDeployDone(true);
          break;
        }
        case 'eventsSummary':
          setEventSummary(data as EventSummary);
          break;
        case 'health':
          setHealthSummary(data as HealthSummary);
          {
            const hs = data as HealthSummary;
            syncHealthFromSummary(hs.healthy, hs.degraded, hs.unhealthy);
            if (hs.healthy + hs.degraded + hs.unhealthy > 0) setHealthDone(true);
          }
          break;
        case 'backups':
          setBackups(data as BackupInfo[]);
          break;
        case 'secrets':
          setSecrets(data as SecretSummary[]);
          break;
        case 'events':
          setEvents(data as Event[]);
          break;
        case 'clusters':
          setClusterSummary(data as ClusterSummary);
          break;
        case 'plugins':
          setPlugins(data as PluginInfo[]);
          break;
        case 'environments':
          setEnvironments(data as Environment[]);
          break;
        case 'apiKeys':
          setApiKeys(data as ApiKeySummary[]);
          break;
      }
    });

    setFailedEndpoints(failed);
    setTotalFailure(successCount === 0);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading) {
    return (
      <div className="space-y-8">
        <div className="command-center-shell animate-pulse p-8">
          <div className="h-8 w-48 rounded-lg bg-slate-800/80" />
          <div className="mt-8 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="h-32 rounded-2xl bg-slate-800/60" />
            ))}
          </div>
        </div>
        <div className="overview-section-shell p-6">
          <div className="grid grid-cols-2 gap-3 lg:grid-cols-5 xl:grid-cols-5">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="skeleton h-24 rounded-2xl" />
            ))}
          </div>
        </div>
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
          <div className="glass-panel-card skeleton h-48" />
          <div className="glass-panel-card skeleton h-48" />
        </div>
      </div>
    );
  }

  if (totalFailure) {
    return (
      <div>
        <PlatformStatusPanel
          platform={capabilities?.platform ?? null}
          ready={ready}
          sseConnected={sseConnected}
          loading={platformLoading}
        />
        <EmptyState
          icon={<WifiOff size={48} />}
          title="API unreachable"
          description="Could not load dashboard data. Check that aether serve is running and your bearer token is valid."
          action={
            <button
              type="button"
              onClick={() => void load()}
              className="inline-flex items-center gap-2 rounded-xl bg-aether px-4 py-2 text-sm font-medium text-white hover:bg-aether/90 transition-colors"
            >
              <RefreshCw className="h-4 w-4" />
              Retry
            </button>
          }
        />
      </div>
    );
  }

  const healthy = healthSummary?.healthy ?? 0;
  const degraded = (healthSummary?.degraded ?? 0) + (healthSummary?.unhealthy ?? 0);
  const aetherManagedCount = countAetherManaged(workloads);
  const clusterDiscoveredCount = workloads.filter((w) => w.source === 'cluster').length;
  const k8sApps = workloads.filter(isK8sApplication);
  const showK8sControlCenter = k8sApps.length > 0 || (clusterSummary?.enabled ?? false);
  const isEmptyPlatform = aetherManagedCount === 0;

  const setupHints = platformSetupNeeded(capabilities?.platform ?? null);

  const clusterContext = clusterSummary?.enabled
    ? {
        connected: clusterSummary.connected,
        clusterCount: clusterSummary.cluster_count,
      }
    : {
        connected: false,
        clusterCount: 0,
        mode: capabilities?.platform?.workloadState.backend === 'postgresql' ? 'PostgreSQL HA' : 'Local mode · No kubeconfig',
      };

  const quickLinks: QuickLink[] = [
    { label: 'Platform & HA', testId: 'overview-platform-quick-link', onClick: () => onNavigate('platform') },
    { label: 'AI engine', testId: 'overview-ai-quick-link', onClick: () => onNavigate('ai') },
    { label: 'Intelligence', testId: 'overview-intelligence-quick-link', onClick: () => onNavigate('intelligence') },
    { label: 'Ops copilot', testId: 'overview-copilot-quick-link', onClick: () => onNavigate('copilot') },
    { label: 'Events feed', testId: 'overview-events-quick-link', onClick: () => onNavigate('events') },
    { label: 'Alerts & webhooks', testId: 'overview-alerts-quick-link', onClick: () => onNavigate('alerts') },
    {
      label: 'GitOps sync',
      testId: 'overview-gitops-quick-link',
      onClick: () => goFiltered('gitops', focusedWorkload ? { workload: focusedWorkload } : undefined),
    },
    {
      label: 'Audit trail',
      testId: 'overview-audit-quick-link',
      onClick: () => goFiltered('audit', focusedWorkload ? { workload: focusedWorkload } : undefined),
    },
    {
      label: 'Drift detection',
      testId: 'overview-drift-quick-link',
      onClick: () => goFiltered('drift', focusedWorkload ? { workload: focusedWorkload } : undefined),
    },
    { label: 'Fleet overview', testId: 'overview-fleet-quick-link', onClick: () => onNavigate('fleet') },
    { label: 'Validate YAML', testId: 'overview-validate-quick-link', onClick: () => goFiltered('workloads', { validate: '1' }) },
    { label: 'Cost estimation', testId: 'overview-cost-quick-link', onClick: () => onNavigate('cost') },
    { label: 'Runtime affinity', testId: 'overview-affinity-quick-link', onClick: () => onNavigate('affinity') },
    { label: 'Confidential computing', testId: 'overview-confidential-quick-link', onClick: () => onNavigate('confidential') },
    {
      label: 'Trust & attestation',
      testId: 'overview-trust-quick-link',
      onClick: () =>
        goFiltered('workloads', focusedWorkload ? { workload: focusedWorkload, tab: 'trust' } : { tab: 'trust' }),
    },
    { label: 'Compose import', testId: 'overview-compose-quick-link', onClick: () => onNavigate('compose') },
    { label: 'Scheduler', testId: 'overview-scheduler-quick-link', onClick: () => onNavigate('scheduler') },
    { label: 'OpenAPI explorer', testId: 'overview-openapi-quick-link', onClick: () => onNavigate('openapi') },
    { label: 'SLA compliance', testId: 'overview-sla-quick-link', onClick: () => onNavigate('sla') },
    { label: 'Dependencies', testId: 'overview-deps-quick-link', onClick: () => onNavigate('deps') },
    { label: 'Secrets vault', testId: 'overview-secrets-quick-link', onClick: () => onNavigate('secrets') },
    {
      label: 'Metrics & Grafana',
      testId: 'overview-metrics-quick-link',
      onClick: () => goFiltered('metrics', focusedWorkload ? { workload: focusedWorkload } : undefined),
    },
    { label: 'Health monitor', testId: 'overview-health-quick-link', onClick: () => onNavigate('health') },
    { label: 'Policy check', testId: 'overview-policy-quick-link', onClick: () => onNavigate('policy') },
    { label: 'Plugins', testId: 'overview-plugins-quick-link', onClick: () => onNavigate('plugins') },
    { label: 'Backups', testId: 'overview-backups-quick-link', onClick: () => onNavigate('backups') },
    { label: 'Visual editor', testId: 'overview-editor-quick-link', onClick: () => onNavigate('editor') },
    { label: 'Workload templates', testId: 'overview-templates-quick-link', onClick: () => onNavigate('templates') },
    { label: 'Access control', testId: 'overview-rbac-quick-link', onClick: () => onNavigate('rbac') },
    { label: 'Cluster browser', testId: 'overview-clusters-quick-link', onClick: () => onNavigate('clusters') },
    { label: 'Intent violations', testId: 'overview-intent-violations-quick-link', onClick: () => goFiltered('events', { category: 'intent-violation' }) },
    { label: 'Discovered workloads', testId: 'overview-discovered-quick-link', onClick: () => goFiltered('workloads', { source: 'cluster' }) },
    { label: 'Environment promotion', testId: 'overview-envs-quick-link', onClick: () => onNavigate('envs') },
  ];

  return (
    <div>
      <CommandCenterBriefing
        onNavigate={onNavigate}
        refreshKey={refreshKey}
        clusterContext={clusterContext}
      />
      <CommandCenterIntentSla refreshKey={refreshKey} />
      <CommandCenterNextActions onNavigate={onNavigate} refreshKey={refreshKey} />

      <PlatformStatusPanel
        platform={capabilities?.platform ?? null}
        ready={ready}
        sseConnected={sseConnected}
        loading={platformLoading}
      />

      {focusedWorkload ? (
        <WorkloadContextBanner
          testId="overview-workload-context"
          workload={focusedWorkload}
          description="Dashboard context"
        >
          <WorkloadScopedCrossLinks
            workload={focusedWorkload}
            prefix="overview"
            showDrift
            showAudit
            showGitops
            showMetrics
          />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('clusters'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-clusters-link"
          >
            Clusters →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('rbac'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-rbac-link"
          >
            RBAC →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('copilot'), { workload: focusedWorkload, q: `Summarize ${focusedWorkload}` })}
            className="text-aether hover:underline"
            data-testid="overview-copilot-link"
          >
            Copilot →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('intelligence'), { workload: focusedWorkload, tab: 'predictions' })}
            className="text-aether hover:underline"
            data-testid="overview-intelligence-link"
          >
            Intelligence →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('deps'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-deps-link"
          >
            Dependencies →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-compose-link"
          >
            Compose →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="overview-fleet-link"
          >
            Fleet →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      {setupHints.length > 0 ? (
        <div className="glass-alert-info mb-6">
          <p className="text-sm font-medium text-blue-200 mb-2">Platform setup recommended</p>
          <ul className="text-sm text-blue-100/90 space-y-1 list-disc list-inside">
            {setupHints.map((hint) => (
              <li key={hint}>{hint}</li>
            ))}
          </ul>
          <button
            type="button"
            onClick={() => onNavigate('platform')}
            className="mt-3 text-sm font-medium text-aether hover:text-blue-300 transition-colors"
          >
            Open Platform &amp; HA →
          </button>
        </div>
      ) : null}

      {failedEndpoints.length > 0 ? (
        <div className="glass-alert-warn mb-6 flex flex-wrap items-center gap-3">
          <AlertTriangle className="h-4 w-4 text-amber-400 shrink-0" />
          <p className="text-sm text-amber-200 flex-1 min-w-0">
            Some sections failed to load:{' '}
            {failedEndpoints.map((e) => ENDPOINT_LABELS[e]).join(', ')}.
          </p>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-1.5 rounded-lg border border-amber-500/40 px-3 py-1.5 text-xs font-medium text-amber-200 hover:bg-amber-500/15 transition-colors"
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Retry
          </button>
        </div>
      ) : null}

      {showK8sControlCenter ? (
        <K8sControlCenter
          username={username}
          workloads={workloads}
          clusterSummary={clusterSummary}
          onNavigate={onNavigate}
        />
      ) : null}

      {isEmptyPlatform ? (
        <div className="mb-6 space-y-4">
          <OnboardingStrip
            hasWorkloads={deployDone || aetherManagedCount > 0}
            hasValidated={specValidated}
            hasHealthChecks={healthDone || (healthSummary?.healthy ?? 0) + (healthSummary?.degraded ?? 0) + (healthSummary?.unhealthy ?? 0) > 0}
            onNavigate={onNavigate}
            onDeploy={() => goFiltered('workloads', { deploy: '1' })}
            onValidate={() => goFiltered('workloads', { validate: '1' })}
          />
          <EmptyState
            icon={<Rocket size={48} />}
            title="Deploy your first workload"
            description="Aether is connected but no workloads are running yet. Deploy a YAML spec, use the visual editor, or import Docker Compose."
            action={
              <div className="flex flex-wrap justify-center gap-2">
                <button
                  type="button"
                  onClick={() => goFiltered('workloads', { deploy: '1' })}
                  className="btn-primary rounded-xl px-4 py-2 text-sm font-medium"
                >
                  Deploy YAML
                </button>
                <button
                  type="button"
                  onClick={() => onNavigate('editor')}
                  className="quick-link-chip"
                >
                  Visual Editor
                </button>
                <button
                  type="button"
                  onClick={() => onNavigate('templates')}
                  className="quick-link-chip"
                >
                  Try a template
                </button>
                <button
                  type="button"
                  onClick={() => onNavigate('compose')}
                  className="quick-link-chip"
                >
                  Import Compose
                </button>
              </div>
            }
          />
        </div>
      ) : null}

      <OverviewFleetSnapshot
        workloadsCount={workloads.length}
        aetherManagedCount={aetherManagedCount}
        clusterDiscoveredCount={clusterDiscoveredCount}
        sseConnected={sseConnected}
        healthy={healthy}
        degraded={degraded}
        pluginsCount={plugins.length}
        environmentsCount={environments.length}
        apiKeysCount={apiKeys.length}
        onNavigate={onNavigate}
      />

      <section className="overview-section-shell mb-8 p-6 sm:p-8">
        <div className="overview-section-header">
          <p className="section-label">Operations</p>
          <h2 className="section-title">Metrics</h2>
          <p className="section-subtitle">Platform inventory and health signals</p>
        </div>
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-3 xl:grid-cols-5">
          <button type="button" onClick={() => goFiltered('workloads', { source: 'aether' })} className="text-left" data-testid="overview-aether-stat">
            <StatCard title="Aether workloads" value={aetherManagedCount} color="orange" icon={<LayoutDashboard size={16} />} compact isEmpty={aetherManagedCount === 0} />
          </button>
          <button type="button" onClick={() => onNavigate('clusters')} className="text-left" data-testid="overview-clusters-stat">
            <StatCard title="Clusters" value={clusterSummary?.cluster_count ?? 0} color="blue" icon={<Activity size={16} />} compact isEmpty={(clusterSummary?.cluster_count ?? 0) === 0} />
          </button>
          <button type="button" onClick={() => goFiltered('health', { status: 'healthy' })} className="text-left" data-testid="overview-healthy-stat">
            <StatCard title="Healthy" value={healthy} color="green" icon={<Activity size={16} />} compact isEmpty={healthy === 0} />
          </button>
          <button type="button" onClick={() => goFiltered('health', { status: 'degraded' })} className="text-left" data-testid="overview-degraded-stat">
            <StatCard title="Degraded" value={degraded} color="red" icon={<AlertTriangle size={16} />} compact isEmpty={degraded === 0} />
          </button>
          <button
            type="button"
            onClick={() =>
              goFiltered(
                'events',
                (eventSummary?.unacknowledged ?? 0) > 0 ? { severity: 'warning' } : undefined,
              )
            }
            className="text-left"
            data-testid="overview-events-stat"
          >
            <StatCard title="Events" value={eventSummary?.total_events ?? 0} color="blue" icon={<Calendar size={16} />} compact isEmpty={(eventSummary?.total_events ?? 0) === 0} />
          </button>
          <button onClick={() => onNavigate('backups')} className="text-left" data-testid="overview-backups-stat">
            <StatCard title="Backups" value={backups.length} color="purple" icon={<Shield size={16} />} compact isEmpty={backups.length === 0} />
          </button>
          <button type="button" onClick={() => onNavigate('secrets')} className="text-left" data-testid="overview-secrets-stat">
            <StatCard title="Secrets" value={secrets.length} color="yellow" icon={<Lock size={16} />} compact isEmpty={secrets.length === 0} />
          </button>
          <button onClick={() => onNavigate('plugins')} className="text-left" data-testid="overview-plugins-stat">
            <StatCard title="Plugins" value={plugins.length} color="blue" icon={<Boxes size={16} />} compact isEmpty={plugins.length === 0} />
          </button>
          <button onClick={() => onNavigate('envs')} className="text-left" data-testid="overview-envs-stat">
            <StatCard title="Environments" value={environments.length} color="green" icon={<Workflow size={16} />} compact isEmpty={environments.length === 0} />
          </button>
          <button type="button" onClick={() => onNavigate('rbac')} className="text-left" data-testid="overview-keys-stat">
            <StatCard title="API Keys" value={apiKeys.length} color="purple" icon={<KeySquare size={16} />} compact isEmpty={apiKeys.length === 0} />
          </button>
        </div>
      </section>

      <OverviewQuickAccess links={quickLinks} />

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="glass-panel-card">
          <h2 className="panel-title mb-4">Clusters</h2>
          {failedEndpoints.includes('clusters') ? (
            <EmptyState
              icon={<WifiOff size={40} />}
              title="Cluster data unavailable"
              description="Could not load cluster summary from the API."
              action={
                <button type="button" onClick={() => void load()} className="text-sm text-aether hover:underline">
                  Retry
                </button>
              }
            />
          ) : !clusterSummary || !clusterSummary.enabled ? (
            <EmptyState
              icon={<Activity size={48} />}
              title="No kubeconfig clusters"
              description="Connect a kubeconfig to discover cluster contexts, or deploy workloads locally with Podman or Docker."
            />
          ) : (
            <div className="space-y-4">
              <div
                className={`rounded-lg border px-4 py-3 text-sm ${
                  clusterSummary.connected
                    ? 'border-emerald-500/20 bg-emerald-500/10 text-emerald-300'
                    : 'border-amber-500/20 bg-amber-500/10 text-amber-300'
                }`}
              >
                {clusterSummary.connected
                  ? `${clusterSummary.healthy_clusters}/${clusterSummary.cluster_count} cluster contexts reachable`
                  : clusterSummary.error ?? 'Kubernetes cluster discovery unavailable'}
              </div>
              <div className="grid grid-cols-3 gap-3">
                <StatCard title="Clusters" value={clusterSummary.cluster_count} color="blue" compact isEmpty={clusterSummary.cluster_count === 0} />
                <StatCard title="Reachable" value={clusterSummary.healthy_clusters} color="green" compact isEmpty={clusterSummary.healthy_clusters === 0} />
                <StatCard title="K8s Workloads" value={clusterSummary.workload_count} color="orange" compact isEmpty={clusterSummary.workload_count === 0} />
              </div>
              {clusterSummary.clusters.length > 0 && (
                <div className="space-y-2">
                  {clusterSummary.clusters.slice(0, 6).map((cluster) => (
                    <div key={cluster.name} className="flex items-center justify-between rounded-xl border border-slate-800/60 bg-[#11151C] px-3 py-2 text-sm">
                      <div>
                        <div className="text-slate-200 font-medium">{cluster.name}</div>
                        <div className="text-slate-500 text-xs">{cluster.version ?? cluster.server ?? 'unreachable'}</div>
                      </div>
                      <div className={cluster.reachable ? 'text-emerald-400' : 'text-amber-400'}>
                        {cluster.reachable ? 'reachable' : 'offline'}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        <div className="glass-panel-card">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="panel-title">Events</h2>
            <button
              type="button"
              onClick={() => goFiltered('events')}
              className="text-sm text-aether hover:text-aether-light transition-colors"
            >
              View all
            </button>
          </div>
          {failedEndpoints.includes('events') ? (
            <EmptyState
              icon={<WifiOff size={40} />}
              title="Events unavailable"
              description="Could not load events from the API."
              action={
                <button type="button" onClick={() => void load()} className="text-sm text-aether hover:underline">
                  Retry
                </button>
              }
            />
          ) : events.length === 0 ? (
            <EmptyState icon={<Inbox size={48} />} title="No events" description="No events have been recorded yet" />
          ) : (
            <div className="space-y-3 max-h-96 overflow-auto">
              {events.slice(0, 20).map((ev, i) => (
                <div
                  key={i}
                  data-testid={`overview-recent-event-${i}`}
                  className="flex items-start gap-3 rounded-xl border border-slate-800/50 bg-[#11151C] p-3"
                >
                  <SeverityBadge severity={ev.severity} />
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium text-slate-200 truncate">{ev.title}</div>
                    <div className="text-xs text-slate-500 mt-0.5">{ev.message}</div>
                    <div className="text-xs text-slate-600 mt-1 flex flex-wrap items-center gap-2">
                      {formatTimestamp(ev.timestamp)}
                      {ev.workload ? (
                        <button
                          type="button"
                          className="text-aether hover:underline"
                          onClick={() => goFiltered('events', { workload: ev.workload! })}
                        >
                          {ev.workload}
                        </button>
                      ) : null}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="glass-panel-card">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="panel-title">Health</h2>
            <button
              type="button"
              onClick={() => onNavigate('health')}
              className="text-sm text-aether hover:text-aether-light transition-colors"
            >
              View all
            </button>
          </div>
          {failedEndpoints.includes('health') ? (
            <EmptyState
              icon={<WifiOff size={40} />}
              title="Health data unavailable"
              description="Could not load health summary from the API."
              action={
                <button type="button" onClick={() => void load()} className="text-sm text-aether hover:underline">
                  Retry
                </button>
              }
            />
          ) : healthSummary ? (
            <div className="grid grid-cols-2 gap-3">
              <StatCard title="Healthy" value={healthSummary.healthy} color="green" compact isEmpty={healthSummary.healthy === 0} />
              <StatCard title="Degraded" value={healthSummary.degraded} color="yellow" compact isEmpty={healthSummary.degraded === 0} />
              <button
                type="button"
                onClick={() => goFiltered('health', { status: 'unhealthy' })}
                className="text-left"
                data-testid="overview-unhealthy-stat"
              >
                <StatCard title="Unhealthy" value={healthSummary.unhealthy} color="red" compact isEmpty={healthSummary.unhealthy === 0} />
              </button>
              <StatCard title="Unknown" value={healthSummary.unknown} color="blue" compact isEmpty={healthSummary.unknown === 0} />
              <StatCard title="Circuits Open" value={healthSummary.circuits_open} color="orange" compact isEmpty={healthSummary.circuits_open === 0} />
            </div>
          ) : (
            <EmptyState icon={<Activity size={48} />} title="No health data" description="Health monitoring is not active" />
          )}
        </div>
      </div>
    </div>
  );
}

// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router';
import {
  AlertTriangle,
  Bell,
  Boxes,
  ChevronDown,
  ChevronRight,
  CircleCheck,
  ClipboardCheck,
  FileCode2,
  RefreshCw,
  Rocket,
  Server,
  Siren,
  Wrench,
  type LucideIcon,
} from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatRelativeTime, formatTimestamp } from '../../utils/formatters';
import { needsAttention, workloadCondition } from '../../utils/workloadStatus';
import type { AttentionSources } from '../../utils/attention';
import AttentionPanel from '../overview/AttentionPanel';
import type { AttentionQueue } from '../../types/operator';
import WelcomeCard from '../overview/WelcomeCard';
import FleetShape from '../overview/FleetShape';
import type { Tone } from '../layout/PageHero';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { useAuth } from '../../contexts/AuthContext';
import type { AppView } from '../../types/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { hasValidatedSpec, hasDeployedWorkload, hasReviewedHealth, syncDeployFromWorkloads, syncHealthFromSummary } from '../../utils/onboardingState';
import { countAetherManaged } from '../../utils/workloadFilters';
import CommandCenterBriefing from '../CommandCenterBriefing';
import CommandCenterNextActions from '../CommandCenterNextActions';
import AuroraPage from '../layout/AuroraPage';
import { SectionHeader } from '../layout/SectionHeader';
import { SeverityBadge } from '../Badge';
import OnboardingStrip from '../OnboardingStrip';
import EmptyState from '../EmptyState';
import PlatformStatusPanel from '../PlatformStatusPanel';
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
  | 'apiKeys'
  | 'operator';

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
  operator: 'Operations queue',
};

type QuickLink = {
  label: string;
  testId: string;
  onClick: () => void;
};

function OverviewPage({ username = '', onNavigate, sseConnected = false, refreshKey = 0 }: OverviewPageProps) {
  const navigate = useNavigate();
  const [workloadFocus] = useQueryParam('workload');
  const focusedWorkload = workloadFocus.trim();
  const { capabilities, ready, loading: platformLoading } = useServerCapabilities();
  const { canMutate } = useAuth();

  const goFiltered = (view: AppView, params?: Record<string, string>) => {
    navigate(pathWithQuery(viewToPath(view), params ?? {}));
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [eventSummary, setEventSummary] = useState<EventSummary | null>(null);
  const [healthSummary, setHealthSummary] = useState<HealthSummary | null>(null);
  const [operatorQueue, setOperatorQueue] = useState<AttentionQueue | null>(null);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [events, setEvents] = useState<Event[]>([]);
  const [clusterSummary, setClusterSummary] = useState<ClusterSummary | null>(null);
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [environments, setEnvironments] = useState<Environment[]>([]);
  const [apiKeys, setApiKeys] = useState<ApiKeySummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasLoadedOnce, setHasLoadedOnce] = useState(false);
  const [failedEndpoints, setFailedEndpoints] = useState<OverviewEndpoint[]>([]);
  const [totalFailure, setTotalFailure] = useState(false);
  const [failureDetail, setFailureDetail] = useState<string | undefined>(undefined);
  const [specValidated, setSpecValidated] = useState(() => hasValidatedSpec());
  const [deployDone, setDeployDone] = useState(() => hasDeployedWorkload());
  const [healthDone, setHealthDone] = useState(() => hasReviewedHealth());
  const [loadedAt, setLoadedAt] = useState<Date | null>(null);

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
      apiFetchSettled<AttentionQueue>('/attention'),
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
      'operator',
    ];

    const failed: OverviewEndpoint[] = [];
    let successCount = 0;
    let firstFailure: string | undefined;

    requests.forEach((result, i) => {
      const key = keys[i];
      if (result.status === 'rejected' || !result.value.ok) {
        failed.push(key);
        if (result.status === 'fulfilled' && !result.value.ok) firstFailure ??= result.value.detail;
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
        case 'operator':
          setOperatorQueue(data as AttentionQueue);
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
    setFailureDetail(firstFailure);
    setTotalFailure(successCount === 0);
    setLoadedAt(new Date());
    setLoading(false);
    setHasLoadedOnce(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refreshKey intentionally triggers a refetch in place (see App.tsx SSE handler), not a remount.
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading && !hasLoadedOnce) {
    return (
      <AuroraPage view="overview">
        <PageLoading rows={5} />
      </AuroraPage>
    );
  }

  if (totalFailure) {
    return (
      <AuroraPage view="overview">
        <PageLoadError
          title="API unreachable"
          description="Could not load dashboard data. Check that aether serve is running and your bearer token is valid."
          detail={failureDetail}
          onRetry={load}
        />
        <PlatformStatusPanel
          platform={capabilities?.platform ?? null}
          ready={ready}
          sseConnected={sseConnected}
          loading={platformLoading}
        />
      </AuroraPage>
    );
  }

  const healthy = healthSummary?.healthy ?? 0;
  const degraded = (healthSummary?.degraded ?? 0) + (healthSummary?.unhealthy ?? 0);
  const aetherManagedCount = countAetherManaged(workloads);
  const clusterDiscoveredCount = workloads.filter((w) => w.source === 'cluster').length;
  // A failed request says nothing about whether the platform is empty.
  const isEmptyPlatform = !failedEndpoints.includes('workloads') && aetherManagedCount === 0;

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
    { label: 'Workloads', testId: 'overview-workloads-quick-link', onClick: () => onNavigate('workloads') },
    { label: 'Deploy YAML', testId: 'overview-validate-quick-link', onClick: () => goFiltered('workloads', { deploy: '1' }) },
    { label: 'Visual editor', testId: 'overview-editor-quick-link', onClick: () => onNavigate('editor') },
    { label: 'Observability', testId: 'overview-health-quick-link', onClick: () => onNavigate('observability') },
    { label: 'Intelligence', testId: 'overview-intelligence-quick-link', onClick: () => onNavigate('intelligence') },
    { label: 'Fleet', testId: 'overview-fleet-quick-link', onClick: () => onNavigate('fleet') },
    { label: 'Clusters', testId: 'overview-clusters-quick-link', onClick: () => onNavigate('clusters') },
    { label: 'Settings', testId: 'overview-platform-quick-link', onClick: () => onNavigate('settings') },
  ];

  const loaded = <T,>(key: OverviewEndpoint, value: T): T | null => (failedEndpoints.includes(key) ? null : value);
  const sources: AttentionSources = {
    workloads: loaded('workloads', workloads),
    events: loaded('events', events),
    secrets: loaded('secrets', secrets),
    health: loaded('health', healthSummary),
    operator: loaded('operator', operatorQueue),
  };

  const runtimeSwatches = RUNTIME_SWATCHES.map(({ runtime, label, tone }) => {
    const count = sources.workloads?.filter((w) => w.runtime.toLowerCase() === runtime).length;
    return {
      label: count === undefined ? label : `${label} ${count}`,
      tone,
      onSelect: () => goFiltered('workloads', { q: runtime }),
    };
  });

  const kpis: Array<{ label: string; value: number | null; tone?: 'danger' | 'success'; icon: LucideIcon; iconTone: Tone; testId: string; onClick: () => void }> = [
    { label: 'Workloads', value: sources.workloads?.length ?? null, icon: Boxes, iconTone: 'sky', testId: 'overview-kpi-workloads', onClick: () => onNavigate('workloads') },
    {
      icon: CircleCheck,
      iconTone: 'emerald',
      label: 'Running',
      value: sources.workloads?.filter((w) => workloadCondition(w.status) === 'running').length ?? null,
      tone: 'success',
      testId: 'overview-kpi-running',
      onClick: () => goFiltered('workloads', { status: 'running' }),
    },
    {
      icon: Siren,
      iconTone: 'pink',
      label: 'Workloads needing attention',
      value: sources.workloads?.filter((w) => needsAttention(w.status)).length ?? null,
      tone: 'danger',
      testId: 'overview-kpi-attention',
      onClick: () => goFiltered('workloads', { status: 'attention' }),
    },
    {
      icon: Bell,
      iconTone: 'amber',
      label: 'Unacknowledged events',
      value: loaded('eventsSummary', eventSummary)?.unacknowledged ?? null,
      testId: 'overview-kpi-events',
      onClick: () => goFiltered('events', { ack: 'open' }),
    },
    {
      icon: Server,
      iconTone: 'teal',
      label: 'Clusters',
      value: loaded('clusters', clusterSummary)?.cluster_count ?? null,
      testId: 'overview-kpi-clusters',
      onClick: () => onNavigate('clusters'),
    },
  ];

  return (
    <AuroraPage view="overview" accent="sky" swatches={runtimeSwatches}>
      {failedEndpoints.length > 0 ? (
        <div className="glass-alert-warn flex flex-wrap items-center gap-3">
          <AlertTriangle className="h-4 w-4 text-warning shrink-0" />
          <p className="text-sm text-warning flex-1 min-w-0">
            Some sections failed to load:{' '}
            {failedEndpoints.map((e) => ENDPOINT_LABELS[e]).join(', ')}.
          </p>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-1.5 rounded-full border border-warning/40 px-3 py-1.5 text-xs font-medium text-warning hover:bg-warning/10 transition-colors"
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Retry
          </button>
        </div>
      ) : null}

      {!isEmptyPlatform || workloads.length > 0 ? (
        <>
          <AttentionPanel sources={sources} checkedAt={loadedAt} refreshing={loading} onRefresh={() => void load()} />

          <div className="kpi-grid" data-testid="overview-apple-highlights">
            {kpis.map((kpi) => (
              <button key={kpi.label} type="button" className="kpi" onClick={kpi.onClick} data-testid={kpi.testId}>
                <span className="kpi-icon" data-tone={kpi.iconTone} aria-hidden>
                  <kpi.icon />
                </span>
                <span className={`kpi-value ${kpi.value && kpi.tone === 'danger' ? '!text-danger' : ''}`}>
                  {kpi.value ?? '—'}
                </span>
                <span className="kpi-label">{kpi.value === null ? `${kpi.label} (not loaded)` : kpi.label}</span>
              </button>
            ))}
          </div>

          <FleetShape workloads={sources.workloads} onRuntime={(runtime) => goFiltered('workloads', { q: runtime })} />

          <QuickActions
            canMutate={canMutate}
            onDeploy={() => goFiltered('workloads', { deploy: '1' })}
            onValidate={() => goFiltered('workloads', { validate: '1' })}
            onNavigate={onNavigate}
          />

          <WelcomeCard />

          <CommandCenterBriefing onNavigate={onNavigate} refreshKey={refreshKey} clusterContext={clusterContext} />

          <div className="grid items-start gap-5 xl:grid-cols-2">
            <RecentEvents events={sources.events} onRetry={() => void load()} goFiltered={goFiltered} />
            <CommandCenterNextActions onNavigate={onNavigate} refreshKey={refreshKey} />
          </div>
        </>
      ) : null}

      {isEmptyPlatform ? <WelcomeCard /> : null}

      {isEmptyPlatform ? (
        <section className="apple-chapter space-y-8">
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
            description="One YAML spec. Every runtime — Podman, Kubernetes, and KubeVirt."
            action={
              <div className="flex flex-wrap justify-center gap-3">
                <button
                  type="button"
                  onClick={() => goFiltered('workloads', { deploy: '1' })}
                  className="btn-primary inline-flex items-center gap-2"
                >
                  Deploy YAML
                </button>
                <button type="button" onClick={() => onNavigate('editor')} className="btn-secondary">
                  Visual Editor
                </button>
              </div>
            }
          />
        </section>
      ) : null}

      <OverviewExplore
        onNavigate={onNavigate}
        goFiltered={goFiltered}
        focusedWorkload={focusedWorkload}
        capabilities={capabilities}
        ready={ready}
        sseConnected={sseConnected}
        platformLoading={platformLoading}
        setupHints={setupHints}
        clusterSummary={clusterSummary}
        isEmptyPlatform={isEmptyPlatform}
        quickLinks={quickLinks}
        aetherManagedCount={aetherManagedCount}
        healthy={healthy}
        degraded={degraded}
        eventCount={eventSummary?.total_events ?? 0}
      />
    </AuroraPage>
  );
}

const RUNTIME_SWATCHES = [
  { runtime: 'podman', label: 'Podman', tone: 'sky' },
  { runtime: 'docker', label: 'Docker', tone: 'amber' },
  { runtime: 'kubernetes', label: 'Kubernetes', tone: 'violet' },
  { runtime: 'kubevirt', label: 'KubeVirt', tone: 'teal' },
] as const;

function QuickActions({
  canMutate,
  onDeploy,
  onValidate,
  onNavigate,
}: {
  canMutate: boolean;
  onDeploy: () => void;
  onValidate: () => void;
  onNavigate: (view: AppView) => void;
}) {
  return (
    <div className="flex flex-wrap items-center gap-2" role="group" aria-label="Quick actions" data-testid="overview-quick-actions">
      {canMutate ? (
        <button type="button" onClick={onDeploy} className="btn-primary !min-h-9">
          <Rocket className="h-4 w-4" aria-hidden />
          Deploy YAML
        </button>
      ) : null}
      <button type="button" onClick={onValidate} className="btn-quiet">
        <ClipboardCheck className="h-3.5 w-3.5" aria-hidden />
        Validate a spec
      </button>
      <button type="button" onClick={() => onNavigate('editor')} className="btn-quiet">
        <Wrench className="h-3.5 w-3.5" aria-hidden />
        Visual editor
      </button>
      <button type="button" onClick={() => onNavigate('compose')} className="btn-quiet">
        <FileCode2 className="h-3.5 w-3.5" aria-hidden />
        Import Compose
      </button>
    </div>
  );
}

function RecentEvents({
  events,
  onRetry,
  goFiltered,
}: {
  events: Event[] | null;
  onRetry: () => void;
  goFiltered: (view: AppView, params?: Record<string, string>) => void;
}) {
  const newest = [...(events ?? [])].sort((a, b) => b.timestamp.localeCompare(a.timestamp)).slice(0, 6);
  return (
    <section className="glass p-5" aria-labelledby="recent-events-title">
      <div className="mb-2 flex items-center justify-between gap-3">
        <h2 id="recent-events-title" className="text-base font-semibold text-foreground">Recent events</h2>
        <button type="button" onClick={() => goFiltered('events')} className="text-[13px] text-primary hover:underline">
          View all
        </button>
      </div>
      {events === null ? (
        <p className="py-4 text-sm text-muted" role="alert">
          Events could not be loaded.{' '}
          <button type="button" onClick={onRetry} className="text-primary hover:underline">Retry</button>
        </p>
      ) : newest.length === 0 ? (
        <p className="py-4 text-sm text-muted">No events have been recorded yet.</p>
      ) : (
        <ul className="divide-y divide-[var(--border)]">
          {newest.map((ev, i) => (
            <li key={`${ev.timestamp}-${i}`} data-testid={`overview-recent-event-${i}`} className="flex items-start gap-3 py-2.5">
              <span className="w-[4.5rem] shrink-0 pt-0.5"><SeverityBadge severity={ev.severity} /></span>
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium text-foreground">{ev.title}</div>
                <div className="truncate text-xs text-muted">{ev.message}</div>
                {ev.workload ? (
                  <button type="button" className="text-xs text-primary hover:underline" onClick={() => goFiltered('events', { workload: ev.workload! })}>
                    {ev.workload}
                  </button>
                ) : null}
              </div>
              <time className="shrink-0 pt-0.5 text-xs text-subtle" dateTime={ev.timestamp} title={formatTimestamp(ev.timestamp)}>
                {formatRelativeTime(ev.timestamp)}
              </time>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function OverviewExplore({
  onNavigate,
  goFiltered,
  focusedWorkload,
  capabilities,
  ready,
  sseConnected,
  platformLoading,
  setupHints,
  clusterSummary,
  isEmptyPlatform,
  quickLinks,
  aetherManagedCount,
  healthy,
  degraded,
}: {
  onNavigate: (view: AppView) => void;
  goFiltered: (view: AppView, params?: Record<string, string>) => void;
  focusedWorkload?: string;
  capabilities: ReturnType<typeof useServerCapabilities>['capabilities'];
  ready: ReturnType<typeof useServerCapabilities>['ready'];
  sseConnected: boolean;
  platformLoading: boolean;
  setupHints: string[];
  clusterSummary: ClusterSummary | null;
  isEmptyPlatform: boolean;
  quickLinks: QuickLink[];
  aetherManagedCount: number;
  healthy: number;
  degraded: number;
  eventCount: number;
}) {
  const [open, setOpen] = useState(false);

  if (isEmptyPlatform) {
    return null;
  }

  return (
    <section className="apple-chapter space-y-8" data-testid="overview-legacy-details-section">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between gap-3 text-left"
        aria-expanded={open}
        data-testid="overview-legacy-details-toggle"
      >
        <SectionHeader
          className="mb-0 flex-1"
          label="Explore"
          title="More from this fleet"
          description="Platform status, setup hints, and inventory links"
        />
        {open ? <ChevronDown className="h-5 w-5 text-subtle" /> : <ChevronRight className="h-5 w-5 text-subtle" />}
      </button>

      {open ? (
        <div className="mt-4 space-y-12" data-testid="overview-legacy-details">
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
                showMetrics
              />
              {' · '}
              <Link
                to={pathWithQuery(viewToPath('intelligence'), { workload: focusedWorkload, tab: 'predictions' })}
                className="text-primary hover:underline"
                data-testid="overview-intelligence-link"
              >
                Intelligence →
              </Link>
            </WorkloadContextBanner>
          ) : null}

          {setupHints.length > 0 ? (
            <div className="glass-alert-info">
              <p className="text-sm font-medium text-primary mb-2">Platform setup recommended</p>
              <ul className="text-sm text-primary space-y-1 list-disc list-inside">
                {setupHints.map((hint) => (
                  <li key={hint}>{hint}</li>
                ))}
              </ul>
              <button
                type="button"
                onClick={() => onNavigate('platform')}
                className="mt-3 text-sm font-medium text-primary hover:text-primary transition-colors"
              >
                Open Platform &amp; HA →
              </button>
            </div>
          ) : null}

          <section className="space-y-6">
            <SectionHeader
              label="Inventory"
              title="At a glance"
              description="Sparse links into managed inventory"
            />
            <div className="flex flex-wrap gap-x-8 gap-y-4 text-sm">
              <button type="button" onClick={() => goFiltered('workloads', { source: 'aether' })} className="text-left text-muted hover:text-foreground" data-testid="overview-aether-stat">
                <span className="block text-2xl font-semibold tabular-nums text-foreground">{aetherManagedCount}</span>
                Aether workloads
              </button>
              <button type="button" onClick={() => onNavigate('clusters')} className="text-left text-muted hover:text-foreground" data-testid="overview-clusters-stat">
                <span className="block text-2xl font-semibold tabular-nums text-foreground">{clusterSummary?.cluster_count ?? 0}</span>
                Clusters
              </button>
              <button type="button" onClick={() => goFiltered('health', { status: 'healthy' })} className="text-left text-muted hover:text-foreground" data-testid="overview-healthy-stat">
                <span className="block text-2xl font-semibold tabular-nums text-foreground">{healthy}</span>
                Healthy
              </button>
              <button type="button" onClick={() => goFiltered('health', { status: 'degraded' })} className="text-left text-muted hover:text-foreground" data-testid="overview-degraded-stat">
                <span className="block text-2xl font-semibold tabular-nums text-foreground">{degraded}</span>
                Degraded
              </button>
            </div>
          </section>

          <section className="space-y-6">
            <SectionHeader
              label="Navigate"
              title="Explore"
              description="Jump to platform tools — also available from the command palette"
            />
            <div className="flex flex-wrap gap-x-6 gap-y-3">
              {quickLinks.map((link) => (
                <button
                  key={link.label}
                  type="button"
                  onClick={link.onClick}
                  data-testid={link.testId}
                  className="text-sm text-primary hover:underline"
                >
                  {link.label}
                </button>
              ))}
            </div>
          </section>
        </div>
      ) : null}
    </section>
  );
}

export default OverviewPage;

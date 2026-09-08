// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router';
import {
  AlertTriangle,
  Inbox,
  RefreshCw,
  Rocket,
  WifiOff,
  ChevronDown,
  ChevronRight,
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

function OverviewPage({ username = '', onNavigate, sseConnected = false, refreshKey = 0 }: OverviewPageProps) {
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
  const [hasLoadedOnce, setHasLoadedOnce] = useState(false);
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
    setHasLoadedOnce(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refreshKey intentionally triggers a refetch in place (see App.tsx SSE handler), not a remount.
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading && !hasLoadedOnce) {
    return (
      <div className="space-y-8">
        <div className="animate-pulse">
          <div className="h-8 w-48 rounded-lg glass-inset-surface" />
          <div className="mt-8 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="h-32 rounded-2xl glass-inset-surface" />
            ))}
          </div>
        </div>
        <div>
          <div className="grid grid-cols-2 gap-3 lg:grid-cols-5 xl:grid-cols-5">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="skeleton h-24 rounded-2xl" />
            ))}
          </div>
        </div>
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
          <div className="glass skeleton h-48" />
          <div className="glass skeleton h-48" />
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
              className="inline-flex items-center gap-2 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-white hover:bg-primary/90 transition-colors"
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
    { label: 'Workloads', testId: 'overview-workloads-quick-link', onClick: () => onNavigate('workloads') },
    { label: 'Deploy YAML', testId: 'overview-validate-quick-link', onClick: () => goFiltered('workloads', { deploy: '1' }) },
    { label: 'Visual editor', testId: 'overview-editor-quick-link', onClick: () => onNavigate('editor') },
    { label: 'Observability', testId: 'overview-health-quick-link', onClick: () => onNavigate('observability') },
    { label: 'Intelligence', testId: 'overview-intelligence-quick-link', onClick: () => onNavigate('intelligence') },
    { label: 'Fleet', testId: 'overview-fleet-quick-link', onClick: () => onNavigate('fleet') },
    { label: 'Clusters', testId: 'overview-clusters-quick-link', onClick: () => onNavigate('clusters') },
    { label: 'Settings', testId: 'overview-platform-quick-link', onClick: () => onNavigate('settings') },
  ];

  const heroStats = !isEmptyPlatform
    ? [
        { label: 'Workloads', value: workloads.length },
        { label: 'Healthy', value: healthy },
        { label: 'Need attention', value: degraded },
        {
          label: 'Clusters',
          value: clusterSummary?.cluster_count ?? clusterContext.clusterCount ?? 0,
        },
      ]
    : undefined;

  return (
    <AuroraPage
      view="overview"
      accent="sky"
      swatches={[
        { label: 'Podman', tone: 'sky' },
        { label: 'Kubernetes', tone: 'violet' },
        { label: 'KubeVirt', tone: 'teal' },
      ]}
      stats={heroStats}
      statsTestId="overview-apple-highlights"
    >
      {!isEmptyPlatform ? (
        <>
          <section className="apple-chapter">
            <CommandCenterBriefing
              onNavigate={onNavigate}
              refreshKey={refreshKey}
              clusterContext={clusterContext}
            />
          </section>

          <section className="apple-chapter">
            <CommandCenterNextActions onNavigate={onNavigate} refreshKey={refreshKey} />
          </section>

          <section className="apple-chapter">
            <RuntimeOrbitFeature onDeploy={() => goFiltered('workloads', { deploy: '1' })} onOpenEditor={() => onNavigate('editor')} />
          </section>
        </>
      ) : null}

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
            className="inline-flex items-center gap-1.5 rounded-full border border-warning/40 px-3 py-1.5 text-xs font-medium text-warning hover:bg-warning/15 transition-colors"
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Retry
          </button>
        </div>
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
        load={load}
        clusterSummary={clusterSummary}
        isEmptyPlatform={isEmptyPlatform}
        events={events}
        failedEndpoints={failedEndpoints}
        quickLinks={quickLinks}
        aetherManagedCount={aetherManagedCount}
        healthy={healthy}
        degraded={degraded}
        eventCount={eventSummary?.total_events ?? 0}
      />
    </AuroraPage>
  );
}

const ORBIT_RUNTIMES = [
  { id: 'podman', label: 'Podman', short: 'Pd', color: 'var(--rt-podman)' },
  { id: 'k8s', label: 'Kubernetes', short: 'K8', color: 'var(--rt-k8s)' },
  { id: 'kubevirt', label: 'KubeVirt', short: 'Kv', color: 'var(--rt-kubevirt)' },
] as const;

function RuntimeOrbitFeature({ onDeploy, onOpenEditor }: { onDeploy: () => void; onOpenEditor: () => void }) {
  return (
    <div className="feature-orbit-card">
      <div>
        <h2 className="text-[34px] font-semibold tracking-[-0.035em] leading-[1.05]">One spec. Every runtime.</h2>
        <p className="mt-3 max-w-[44ch] text-[17px] leading-[1.45] text-muted">
          Write a single YAML spec and deploy it to Podman, Kubernetes, or KubeVirt. Move it between them later without
          rewriting a line.
        </p>
        <div className="mt-6 flex flex-wrap gap-3">
          <button type="button" onClick={onDeploy} className="btn-primary">
            Deploy YAML
          </button>
          <button type="button" onClick={onOpenEditor} className="btn-secondary">
            Open visual editor
          </button>
        </div>
      </div>
      <div className="feature-orbit-art" aria-hidden>
        <div className="orbit" />
        <div className="core">Æ</div>
        {ORBIT_RUNTIMES.map((rt, i) => {
          const angle = (i / ORBIT_RUNTIMES.length) * Math.PI * 2 - Math.PI / 2;
          const x = 50 + Math.cos(angle) * 36;
          const y = 50 + Math.sin(angle) * 36;
          return (
            <div
              key={rt.id}
              className="node"
              style={{ left: `calc(${x}% - 22px)`, top: `calc(${y}% - 22px)`, background: rt.color }}
              title={rt.label}
            >
              {rt.short}
            </div>
          );
        })}
      </div>
    </div>
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
  failedEndpoints,
  load,
  clusterSummary,
  isEmptyPlatform,
  events,
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
  failedEndpoints: OverviewEndpoint[];
  load: () => Promise<void>;
  clusterSummary: ClusterSummary | null;
  isEmptyPlatform: boolean;
  events: Event[];
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
          description="Platform tools, recent events, and inventory links"
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
              <ul className="text-sm text-primary/90 space-y-1 list-disc list-inside">
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

          <section className="space-y-6">
            <div className="flex items-end justify-between gap-4">
              <SectionHeader
                className="mb-0"
                label="Activity"
                title="Recent events"
                description="Latest signals across the fleet"
              />
              <button
                type="button"
                onClick={() => goFiltered('events')}
                className="text-sm text-primary hover:underline shrink-0"
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
                  <button type="button" onClick={() => void load()} className="text-sm text-primary hover:underline">
                    Retry
                  </button>
                }
              />
            ) : events.length === 0 ? (
              <EmptyState icon={<Inbox size={48} />} title="No events" description="No events have been recorded yet" />
            ) : (
              <div className="divide-y divide-[var(--border)]">
                {events.slice(0, 8).map((ev, i) => (
                  <div
                    key={i}
                    data-testid={`overview-recent-event-${i}`}
                    className="flex items-start gap-4 py-5"
                  >
                    <SeverityBadge severity={ev.severity} />
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium text-foreground truncate">{ev.title}</div>
                      <div className="text-xs text-subtle mt-0.5">{ev.message}</div>
                      <div className="text-xs text-subtle mt-1 flex flex-wrap items-center gap-2">
                        {formatTimestamp(ev.timestamp)}
                        {ev.workload ? (
                          <button
                            type="button"
                            className="text-primary hover:underline"
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
          </section>
        </div>
      ) : null}
    </section>
  );
}

export default OverviewPage;

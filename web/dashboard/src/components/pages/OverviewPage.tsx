// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router';
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
} from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import type { AppView } from '../../types/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';
import { hasValidatedSpec, hasDeployedWorkload, hasReviewedHealth, syncDeployFromWorkloads, syncHealthFromSummary } from '../../utils/onboardingState';
import { countAetherManaged } from '../../utils/workloadFilters';
import StatCard from '../StatCard';
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
  onNavigate: (view: AppView) => void;
  sseConnected?: boolean;
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

export default function OverviewPage({ onNavigate, sseConnected = false }: OverviewPageProps) {
  const navigate = useNavigate();
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
      <div>
        <PlatformStatusPanel platform={null} ready={null} sseConnected={sseConnected} loading />
        <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="skeleton h-24 rounded-xl" />
          ))}
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="skeleton h-48 rounded-xl" />
          <div className="skeleton h-48 rounded-xl" />
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
  const isEmptyPlatform = aetherManagedCount === 0;

  const setupHints = platformSetupNeeded(capabilities?.platform ?? null);

  return (
    <div>
      <PlatformStatusPanel
        platform={capabilities?.platform ?? null}
        ready={ready}
        sseConnected={sseConnected}
        loading={platformLoading}
      />

      {setupHints.length > 0 ? (
        <div className="mb-6 rounded-xl border border-blue-500/25 bg-blue-500/10 px-4 py-3">
          <p className="text-sm text-blue-200 mb-2">Platform setup recommended:</p>
          <ul className="text-sm text-blue-100/90 space-y-1 list-disc list-inside">
            {setupHints.map((hint) => (
              <li key={hint}>{hint}</li>
            ))}
          </ul>
          <button
            type="button"
            onClick={() => onNavigate('platform')}
            className="mt-3 text-sm font-medium text-aether hover:underline"
          >
            Open Platform &amp; HA →
          </button>
        </div>
      ) : null}

      {failedEndpoints.length > 0 ? (
        <div className="mb-6 rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 flex flex-wrap items-center gap-3">
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
                  className="rounded-xl bg-aether px-4 py-2 text-sm font-medium text-white hover:bg-aether/90 transition-colors"
                >
                  Deploy YAML
                </button>
                <button
                  type="button"
                  onClick={() => onNavigate('editor')}
                  className="rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:border-aether/40 hover:text-aether transition-colors"
                >
                  Visual Editor
                </button>
                <button
                  type="button"
                  onClick={() => onNavigate('templates')}
                  className="rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:border-aether/40 hover:text-aether transition-colors"
                >
                  Try a template
                </button>
                <button
                  type="button"
                  onClick={() => onNavigate('compose')}
                  className="rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:border-aether/40 hover:text-aether transition-colors"
                >
                  Import Compose
                </button>
              </div>
            }
          />
        </div>
      ) : null}

      <div className="mb-6 grid grid-cols-1 gap-4 xl:grid-cols-3">
        <div className="surface-panel interactive-lift rounded-2xl p-5">
          <div className="mb-3 flex items-center justify-between">
            <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">Runtime Fabric</span>
            <span className="rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2 py-0.5 text-xs text-emerald-300">
              {sseConnected ? 'live' : 'syncing'}
            </span>
          </div>
          <div className="text-2xl font-semibold text-white">{workloads.length + (clusterSummary?.workload_count ?? 0)}</div>
          <p className="mt-1 text-sm text-slate-500">Managed and discovered workloads across every runtime.</p>
        </div>
        <div className="surface-panel interactive-lift rounded-2xl p-5">
          <div className="mb-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">Signal Quality</div>
          <div className="flex items-end gap-3">
            <div className="text-2xl font-semibold text-white">{healthy}</div>
            <div className="pb-1 text-sm text-slate-500">healthy checks</div>
          </div>
          <div className="mt-4 h-2 overflow-hidden rounded-full bg-slate-800">
            <div
              className="h-full rounded-full bg-gradient-to-r from-emerald-400 to-aether"
              style={{ width: `${Math.min(100, Math.max(8, (healthy / Math.max(1, healthy + degraded)) * 100))}%` }}
            />
          </div>
        </div>
        <div className="surface-panel interactive-lift rounded-2xl p-5">
          <div className="mb-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">Control Surface</div>
          <div className="grid grid-cols-3 gap-3 text-center">
            <div className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
              <div className="text-lg font-semibold text-white">{plugins.length}</div>
              <div className="text-[11px] text-slate-500">Plugins</div>
            </div>
            <div className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
              <div className="text-lg font-semibold text-white">{environments.length}</div>
              <div className="text-[11px] text-slate-500">Envs</div>
            </div>
            <div className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
              <div className="text-lg font-semibold text-white">{apiKeys.length}</div>
              <div className="text-[11px] text-slate-500">Keys</div>
            </div>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
        <button type="button" onClick={() => goFiltered('workloads', { source: 'aether' })} className="text-left">
          <StatCard title="Aether workloads" value={aetherManagedCount} color="orange" icon={<LayoutDashboard size={18} />} />
        </button>
        <button type="button" onClick={() => onNavigate('clusters')} className="text-left">
          <StatCard title="Clusters" value={clusterSummary?.cluster_count ?? 0} color="blue" icon={<Activity size={18} />} />
        </button>
        <button onClick={() => onNavigate('health')} className="text-left">
          <StatCard title="Healthy" value={healthy} color="green" icon={<Activity size={18} />} />
        </button>
        <button onClick={() => onNavigate('health')} className="text-left">
          <StatCard title="Degraded" value={degraded} color="red" icon={<AlertTriangle size={18} />} />
        </button>
        <button type="button" onClick={() => goFiltered('events')} className="text-left">
          <StatCard title="Events" value={eventSummary?.total_events ?? 0} color="blue" icon={<Calendar size={18} />} />
        </button>
        <button onClick={() => onNavigate('backups')} className="text-left">
          <StatCard title="Backups" value={backups.length} color="purple" icon={<Shield size={18} />} />
        </button>
        <button onClick={() => onNavigate('secrets')} className="text-left">
          <StatCard title="Secrets" value={secrets.length} color="yellow" icon={<Lock size={18} />} />
        </button>
        <button onClick={() => onNavigate('plugins')} className="text-left">
          <StatCard title="Plugins" value={plugins.length} color="blue" icon={<Boxes size={18} />} />
        </button>
        <button onClick={() => onNavigate('envs')} className="text-left">
          <StatCard title="Environments" value={environments.length} color="green" icon={<Workflow size={18} />} />
        </button>
        <button type="button" onClick={() => onNavigate('rbac')} className="text-left">
          <StatCard title="API Keys" value={apiKeys.length} color="purple" icon={<KeySquare size={18} />} />
        </button>
      </div>

      <div className="mb-6 flex flex-wrap gap-2">
        <span className="w-full text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500 mb-1">Quick links</span>
        {(
          [
            { label: 'Platform & HA', onClick: () => onNavigate('platform') },
            { label: 'Alerts & webhooks', onClick: () => onNavigate('alerts') },
            { label: 'GitOps sync', onClick: () => onNavigate('gitops') },
            { label: 'Audit trail', onClick: () => goFiltered('audit') },
            { label: 'Drift detection', onClick: () => onNavigate('drift') },
            { label: 'Fleet overview', onClick: () => onNavigate('fleet') },
            { label: 'Alerts & webhooks', onClick: () => onNavigate('alerts') },
            { label: 'Scheduler', onClick: () => onNavigate('scheduler') },
            { label: 'SLA compliance', onClick: () => onNavigate('sla') },
            { label: 'API explorer', onClick: () => onNavigate('openapi') },
            { label: 'Dependencies', onClick: () => onNavigate('deps') },
            { label: 'Metrics & Grafana', onClick: () => onNavigate('metrics') },
            { label: 'Intent violations', onClick: () => goFiltered('events', { category: 'intent-violation' }) },
            { label: 'Discovered workloads', onClick: () => goFiltered('workloads', { source: 'cluster' }) },
          ] as const
        ).map((link) => (
          <button
            key={link.label}
            type="button"
            onClick={link.onClick}
            className="rounded-xl border border-slate-800 bg-slate-950/50 px-3 py-2 text-sm text-slate-300 hover:border-aether/40 hover:text-aether transition-colors"
          >
            {link.label}
          </button>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="dash-card">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Clusters</h2>
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
              description="Aether did not find any reachable kubeconfig contexts."
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
                <StatCard title="Clusters" value={clusterSummary.cluster_count} color="blue" />
                <StatCard title="Reachable" value={clusterSummary.healthy_clusters} color="green" />
                <StatCard title="K8s Workloads" value={clusterSummary.workload_count} color="orange" />
              </div>
              {clusterSummary.clusters.length > 0 && (
                <div className="space-y-2">
                  {clusterSummary.clusters.slice(0, 6).map((cluster) => (
                    <div key={cluster.name} className="flex items-center justify-between rounded-lg bg-zinc-950/60 px-3 py-2 text-sm">
                      <div>
                        <div className="text-zinc-200 font-medium">{cluster.name}</div>
                        <div className="text-zinc-500 text-xs">{cluster.version ?? cluster.server ?? 'unreachable'}</div>
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

        <div className="dash-card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-zinc-100">Events</h2>
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
                <div key={i} className="flex items-start gap-3 p-3 bg-zinc-950/50 rounded-lg">
                  <SeverityBadge severity={ev.severity} />
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium text-zinc-200 truncate">{ev.title}</div>
                    <div className="text-xs text-zinc-500 mt-0.5">{ev.message}</div>
                    <div className="text-xs text-zinc-600 mt-1">{formatTimestamp(ev.timestamp)}</div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="dash-card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-zinc-100">Health</h2>
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
            <div className="grid grid-cols-2 gap-4">
              <StatCard title="Healthy" value={healthSummary.healthy} color="green" />
              <StatCard title="Degraded" value={healthSummary.degraded} color="yellow" />
              <button
                type="button"
                onClick={() => goFiltered('health', { status: 'unhealthy' })}
                className="text-left"
              >
                <StatCard title="Unhealthy" value={healthSummary.unhealthy} color="red" />
              </button>
              <StatCard title="Unknown" value={healthSummary.unknown} color="blue" />
              <StatCard title="Circuits Open" value={healthSummary.circuits_open} color="orange" />
            </div>
          ) : (
            <EmptyState icon={<Activity size={48} />} title="No health data" description="Health monitoring is not active" />
          )}
        </div>
      </div>
    </div>
  );
}

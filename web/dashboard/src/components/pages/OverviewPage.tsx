import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router';
import { LayoutDashboard, Activity, AlertTriangle, Calendar, Shield, Lock, Inbox, Boxes, KeySquare, Workflow } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import type { AppView } from '../../types/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';
import StatCard from '../StatCard';
import { SeverityBadge } from '../Badge';
import EmptyState from '../EmptyState';
import PlatformStatusPanel from '../PlatformStatusPanel';
import { useServerCapabilities } from '../../contexts/ServerCapabilitiesContext';
import type { WorkloadResponse, EventSummary, HealthSummary, BackupInfo, SecretSummary, Event, ClusterSummary, PluginInfo, Environment, ApiKeySummary } from '../../types/api';

interface OverviewPageProps {
  onNavigate: (view: AppView) => void;
  sseConnected?: boolean;
}

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

  useEffect(() => {
    async function load() {
      const [w, es, hs, b, s, ev, cs, p, envs, keys] = await Promise.all([
        apiFetch<WorkloadResponse[]>('/workloads'),
        apiFetch<EventSummary>('/events/summary'),
        apiFetch<HealthSummary>('/orchestrator/summary'),
        apiFetch<BackupInfo[]>('/backups'),
        apiFetch<SecretSummary[]>('/secrets'),
        apiFetch<Event[]>('/events'),
        apiFetch<ClusterSummary>('/cluster/summary'),
        apiFetch<PluginInfo[]>('/plugins'),
        apiFetch<Environment[]>('/environments'),
        apiFetch<ApiKeySummary[]>('/rbac/keys'),
      ]);
      setWorkloads(w ?? []);
      setEventSummary(es);
      setHealthSummary(hs);
      setBackups(b ?? []);
      setSecrets(s ?? []);
      setEvents(ev ?? []);
      setClusterSummary(cs);
      setPlugins(p ?? []);
      setEnvironments(envs ?? []);
      setApiKeys(keys ?? []);
      setLoading(false);
    }
    load();
  }, []);

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
          <div className="skeleton h-72 rounded-xl" />
          <div className="skeleton h-72 rounded-xl" />
        </div>
      </div>
    );
  }

  const healthy = healthSummary?.healthy ?? 0;
  const degraded = (healthSummary?.degraded ?? 0) + (healthSummary?.unhealthy ?? 0);

  return (
    <div>
      <PlatformStatusPanel
        platform={capabilities?.platform ?? null}
        ready={ready}
        sseConnected={sseConnected}
        loading={platformLoading && !capabilities}
      />
      {clusterSummary?.summary_note ? (
        <div className="mb-5 rounded-xl border border-amber-500/25 bg-amber-500/10 px-4 py-3 text-sm text-amber-100/95 leading-relaxed">
          {clusterSummary.summary_note}
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
          <StatCard title="Workloads" value={workloads.length} color="orange" icon={<LayoutDashboard size={18} />} />
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
          {!clusterSummary || !clusterSummary.enabled ? (
            <EmptyState
              icon={<Activity size={48} />}
              title="No kubeconfig clusters"
              description="Aether did not find any reachable kubeconfig contexts."
            />
          ) : (
            <div className="space-y-4">
              <div className={`rounded-lg border px-4 py-3 text-sm ${
                clusterSummary.connected
                  ? 'border-emerald-500/20 bg-emerald-500/10 text-emerald-300'
                  : 'border-amber-500/20 bg-amber-500/10 text-amber-300'
              }`}>
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
                  {clusterSummary.clusters.slice(0, 6).map(cluster => (
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

        {/* Recent Events */}
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
          {events.length === 0 ? (
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

        {/* Health Summary */}
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
          {healthSummary ? (
            <div className="grid grid-cols-2 gap-4">
              <StatCard title="Healthy" value={healthSummary.healthy} color="green" />
              <StatCard title="Degraded" value={healthSummary.degraded} color="yellow" />
              <StatCard title="Unhealthy" value={healthSummary.unhealthy} color="red" />
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

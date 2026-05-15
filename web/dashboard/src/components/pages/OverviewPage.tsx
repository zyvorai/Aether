import { useState, useEffect } from 'react';
import { LayoutDashboard, Activity, AlertTriangle, Calendar, Shield, Lock, Inbox, Boxes, KeySquare, Workflow } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import type { AppView } from '../../types/api';
import StatCard from '../StatCard';
import { SeverityBadge } from '../Badge';
import EmptyState from '../EmptyState';
import type { WorkloadResponse, EventSummary, HealthSummary, BackupInfo, SecretSummary, Event, ClusterSummary, PluginInfo, Environment, ApiKeySummary } from '../../types/api';

interface OverviewPageProps {
  onNavigate: (view: AppView) => void;
}

export default function OverviewPage({ onNavigate }: OverviewPageProps) {
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
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
      </div>
    );
  }

  const healthy = healthSummary?.healthy ?? 0;
  const degraded = (healthSummary?.degraded ?? 0) + (healthSummary?.unhealthy ?? 0);

  return (
    <div>
      {clusterSummary?.summary_note ? (
        <div className="mb-5 rounded-xl border border-amber-500/25 bg-amber-500/10 px-4 py-3 text-sm text-amber-100/95 leading-relaxed">
          {clusterSummary.summary_note}
        </div>
      ) : null}
      <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
        <button onClick={() => onNavigate('workloads')} className="text-left">
          <StatCard title="Workloads" value={workloads.length} color="orange" icon={<LayoutDashboard size={18} />} />
        </button>
        <StatCard title="Clusters" value={clusterSummary?.cluster_count ?? 0} color="blue" icon={<Activity size={18} />} />
        <button onClick={() => onNavigate('health')} className="text-left">
          <StatCard title="Healthy" value={healthy} color="green" icon={<Activity size={18} />} />
        </button>
        <button onClick={() => onNavigate('health')} className="text-left">
          <StatCard title="Degraded" value={degraded} color="red" icon={<AlertTriangle size={18} />} />
        </button>
        <button onClick={() => onNavigate('events')} className="text-left">
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
        <button onClick={() => onNavigate('rbac')} className="text-left">
          <StatCard title="API Keys" value={apiKeys.length} color="purple" icon={<KeySquare size={18} />} />
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Kubernetes Clusters</h2>
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
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Recent Events</h2>
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
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Health Summary</h2>
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

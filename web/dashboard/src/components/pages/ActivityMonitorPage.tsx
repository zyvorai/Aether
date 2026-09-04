import { withAuroraPage } from '../layout/AuroraPage';
import SectionHubPage from '../SectionHubPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { Activity, Cpu, HardDrive, RefreshCw, AlertTriangle, Zap, Radio } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';
import { useEventStream } from '../../hooks/useEventStream';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { ClusterMetricsSummary, ClusterSummary, Event, WorkloadResponse } from '../../types/api';
import { isK8sApplication } from '../../utils/k8sUx';

type MonitorTab = 'cpu' | 'memory' | 'restarts' | 'errors';

const POLL_MS = 30_000;

function parseCpu(cpu: string): number {
  if (cpu.endsWith('m')) return Number.parseInt(cpu, 10) || 0;
  return Math.round(Number.parseFloat(cpu) * 1000) || 0;
}

function parseMemoryMi(mem: string): number {
  if (mem.endsWith('Mi')) return Number.parseInt(mem, 10) || 0;
  if (mem.endsWith('Gi')) return Math.round(Number.parseFloat(mem) * 1024) || 0;
  return Number.parseInt(mem, 10) || 0;
}

export function ActivityStudio({ forcedTab }: { forcedTab?: MonitorTab } = {}) {
  const navigate = useNavigate();
  const [tabState, setTabState] = useState<MonitorTab>('cpu');
  const tab: MonitorTab = forcedTab ?? tabState;
  const setTab = (next: MonitorTab) => { if (!forcedTab) setTabState(next); };
  const [initialLoading, setInitialLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [loadFailed, setLoadFailed] = useState(false);
  const [liveEnabled, setLiveEnabled] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [clusterSummary, setClusterSummary] = useState<ClusterSummary | null>(null);
  const [metrics, setMetrics] = useState<ClusterMetricsSummary | null>(null);
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [events, setEvents] = useState<Event[]>([]);
  const [selectedCluster, setSelectedCluster] = useState<string>('');
  const selectedClusterRef = useRef('');

  const load = useCallback(async (silent = false) => {
    if (!silent) setInitialLoading(true);
    else setRefreshing(true);
    setLoadFailed(false);

    const [clusterRes, workloadsRes, eventsRes] = await Promise.all([
      apiFetchSettled<ClusterSummary>('/cluster/summary'),
      apiFetchSettled<WorkloadResponse[]>('/workloads'),
      apiFetchSettled<Event[]>('/events'),
    ]);

    if (!clusterRes.ok && !workloadsRes.ok) {
      setLoadFailed(true);
      setInitialLoading(false);
      setRefreshing(false);
      return;
    }

    const summary = clusterRes.ok ? clusterRes.data : null;
    setClusterSummary(summary);
    setWorkloads(workloadsRes.ok ? workloadsRes.data : []);
    setEvents(eventsRes.ok ? eventsRes.data : []);

    let clusterName = selectedClusterRef.current;
    if (!clusterName && summary?.clusters?.[0]?.name) {
      clusterName = summary.clusters[0].name;
      selectedClusterRef.current = clusterName;
      setSelectedCluster(clusterName);
    }

    if (clusterName) {
      const metricsRes = await apiFetchSettled<ClusterMetricsSummary>(
        `/cluster/metrics/summary?cluster=${encodeURIComponent(clusterName)}`,
      );
      setMetrics(metricsRes.ok ? metricsRes.data : null);
    } else {
      setMetrics(null);
    }

    setLastUpdated(new Date());
    setInitialLoading(false);
    setRefreshing(false);
  }, []);

  useEffect(() => {
    void load(false);
  }, [load]);

  useEffect(() => {
    if (!liveEnabled) return;
    const id = window.setInterval(() => void load(true), POLL_MS);
    return () => window.clearInterval(id);
  }, [liveEnabled, load]);

  const onStreamEvent = useCallback(
    (event: { type: string }) => {
      if (!liveEnabled) return;
      if (
        event.type === 'workloadChanged' ||
        event.type === 'healthUpdate' ||
        event.type === 'eventEmitted'
      ) {
        void load(true);
      }
    },
    [liveEnabled, load],
  );

  const { connected: sseConnected } = useEventStream('', onStreamEvent, liveEnabled);

  const k8sApps = useMemo(() => workloads.filter(isK8sApplication), [workloads]);
  const failingApps = useMemo(() => k8sApps.filter((w) => /fail|error|crash/i.test(w.status)), [k8sApps]);
  const warningEvents = useMemo(
    () => events.filter((e) => e.severity === 'warning' || e.severity === 'error').slice(0, 10),
    [events],
  );

  const topCpu = useMemo(() => {
    if (!metrics?.pods) return [];
    return [...metrics.pods].sort((a, b) => parseCpu(b.cpu) - parseCpu(a.cpu)).slice(0, 10);
  }, [metrics]);

  const topMemory = useMemo(() => {
    if (!metrics?.pods) return [];
    return [...metrics.pods].sort((a, b) => parseMemoryMi(b.memory) - parseMemoryMi(a.memory)).slice(0, 10);
  }, [metrics]);

  if (initialLoading) return <PageLoading label="Loading activity monitor…" />;
  if (loadFailed) return <PageLoadError title="Activity monitor unavailable" onRetry={() => void load(false)} />;

  const tabs: { id: MonitorTab; label: string; icon: typeof Cpu }[] = [
    { id: 'cpu', label: 'CPU', icon: Cpu },
    { id: 'memory', label: 'Memory', icon: HardDrive },
    { id: 'restarts', label: 'Restarts', icon: RefreshCw },
    { id: 'errors', label: 'Errors', icon: AlertTriangle },
  ];

  return (
    <div data-testid="activity-monitor-page">
      <PageToolbar onRefresh={() => void load(true)} refreshing={refreshing} />

      <div className="mb-6 flex flex-wrap items-center justify-end gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 text-xs text-muted">
            <input
              type="checkbox"
              checked={liveEnabled}
              onChange={(e) => setLiveEnabled(e.target.checked)}
              className="accent-primary"
            />
            Live refresh
          </label>
          <span
            className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[10px] font-medium uppercase tracking-wide ${
              sseConnected && liveEnabled
                ? 'bg-success/15 text-success border border-success/30'
                : 'glass-inset-surface text-subtle border glass-divider'
            }`}
          >
            <Radio size={12} className={sseConnected && liveEnabled ? 'animate-pulse' : ''} />
            {liveEnabled ? (sseConnected ? 'SSE live' : 'Polling 30s') : 'Paused'}
          </span>
          {lastUpdated && (
            <span className="text-[10px] text-subtle">
              Updated {lastUpdated.toLocaleTimeString()}
            </span>
          )}
          {(clusterSummary?.clusters.length ?? 0) > 1 && (
            <select
              value={selectedCluster}
              onChange={(e) => {
                selectedClusterRef.current = e.target.value;
                setSelectedCluster(e.target.value);
                void load(true);
              }}
              className="glass-input text-xs"
            >
              {(clusterSummary?.clusters ?? []).map((c) => (
                <option key={c.name} value={c.name}>{c.name}</option>
              ))}
            </select>
          )}
        </div>
      </div>

      <section className="mb-10 flex flex-wrap gap-x-10 gap-y-4">
        <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{metrics?.pod_count ?? 0}</div>Pods</div>
        <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{k8sApps.length}</div>Applications</div>
        <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{failingApps.length}</div>Failing apps</div>
        <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{metrics ? `${metrics.total_cpu_millicores}m` : '—'}</div>Cluster CPU</div>
      </section>

      {!forcedTab ? (
      <div className="flex flex-wrap gap-2 mb-6">
        {tabs.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => setTab(id)}
            className={`glass-tab tab-chip inline-flex items-center gap-2 ${tab === id ? 'glass-tab-active tab-chip-active' : ''}`}
          >
            <Icon size={16} />
            {label}
          </button>
        ))}
      </div>
      ) : null}

      <div className="glass">
        {tab === 'cpu' && (
          <div>
            <h3 className="text-sm font-semibold text-muted mb-4">Top CPU pods</h3>
            {topCpu.length === 0 ? (
              <p className="text-sm text-subtle">No metrics available. Connect a cluster with metrics-server.</p>
            ) : (
              <div className="space-y-2">
                {topCpu.map((p) => (
                  <div key={p.name} className="flex items-center justify-between rounded-lg border glass-divider px-3 py-2">
                    <span className="text-sm text-foreground font-mono">{p.name}</span>
                    <span className="text-sm text-primary">{p.cpu}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {tab === 'memory' && (
          <div>
            <h3 className="text-sm font-semibold text-muted mb-4">Top memory pods</h3>
            {topMemory.length === 0 ? (
              <p className="text-sm text-subtle">No metrics available.</p>
            ) : (
              <div className="space-y-2">
                {topMemory.map((p) => (
                  <div key={p.name} className="flex items-center justify-between rounded-lg border glass-divider px-3 py-2">
                    <span className="text-sm text-foreground font-mono">{p.name}</span>
                    <span className="text-sm text-primary">{p.memory}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {tab === 'restarts' && (
          <div>
            <h3 className="text-sm font-semibold text-muted mb-4">Applications needing attention</h3>
            {failingApps.length === 0 ? (
              <p className="text-sm text-success">No failing applications detected.</p>
            ) : (
              <div className="space-y-2">
                {failingApps.map((app) => (
                  <button
                    key={app.name}
                    type="button"
                    onClick={() => navigate(pathWithQuery(viewToPath('applications'), { workload: app.name }))}
                    className="w-full text-left flex items-center justify-between rounded-lg border border-danger/20 bg-danger/15 px-3 py-2 hover:border-danger/40"
                  >
                    <span className="text-sm text-foreground">{app.name}</span>
                    <span className="text-xs text-danger">{app.status}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        )}

        {tab === 'errors' && (
          <div>
            <h3 className="text-sm font-semibold text-muted mb-4">Recent warnings & errors</h3>
            {warningEvents.length === 0 ? (
              <p className="text-sm text-subtle">No recent warning events.</p>
            ) : (
              <div className="space-y-2">
                {warningEvents.map((ev, i) => (
                  <div key={`${ev.timestamp}-${i}`} className="rounded-lg border glass-divider px-3 py-2">
                    <p className="text-sm text-foreground">{ev.message}</p>
                    <p className="text-xs text-subtle mt-1">{ev.workload ?? 'platform'} · {ev.category}</p>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {clusterSummary && (
        <p className="mt-4 text-xs text-subtle">
          {clusterSummary.cluster_count} cluster(s) · {clusterSummary.workload_count} discovered workloads
        </p>
      )}
    </div>
  );
}

function ActivityHubPage() {
  return (
    <SectionHubPage
      links={[
        { view: 'activity-cpu', title: 'CPU', description: 'Top CPU pods across the fleet.', icon: <Cpu className="h-5 w-5" /> },
        { view: 'activity-memory', title: 'Memory', description: 'Top memory consumers.', icon: <HardDrive className="h-5 w-5" /> },
        { view: 'activity-restarts', title: 'Restarts', description: 'Pods with frequent restarts.', icon: <RefreshCw className="h-5 w-5" /> },
        { view: 'activity-errors', title: 'Errors', description: 'Recent error events.', icon: <AlertTriangle className="h-5 w-5" /> },
      ]}
    />
  );
}

export default withAuroraPage('activity', ActivityHubPage);

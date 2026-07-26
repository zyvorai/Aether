// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ChevronDown, ChevronRight, ExternalLink, Globe, Network, Server, Shield } from 'lucide-react';
import { Link, useNavigate } from 'react-router';
import { apiFetch, apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { hubbleNamespaceUrl, hubbleWorkloadUrl } from '../../utils/hubbleLinks';
import { clusterResourceQuery, parseClusterWorkload } from '../../utils/parseClusterWorkload';
import { isK8sApplication } from '../../utils/k8sUx';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge from '../Badge';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
import FleetIntelligenceBrief from '../FleetIntelligenceBrief';
import MultiCloudPanel from '../MultiCloudPanel';
import FederationPlatformPanel from '../FederationPlatformPanel';
import StatCard from '../StatCard';
import type {
  ClusterPodSummary,
  ClusterResourceDetail,
  ClusterSummary,
  WorkloadResponse,
  EdgeAgentRecord,
  FederationPlan,
  PacketWolfStatus,
} from '../../types/api';

interface Integrations {
  hubble_ui_url?: string | null;
  packetwolf_url?: string | null;
  grafana_url?: string | null;
  prometheus_url?: string | null;
}

interface ServerPayload {
  integrations?: Integrations;
}

export default function FleetPage() {
  const navigate = useNavigate();
  const [workloadFocus] = useQueryParam('workload');
  const focusedWorkload = workloadFocus.trim();
  const [summary, setSummary] = useState<ClusterSummary | null>(null);
  const [integrations, setIntegrations] = useState<Integrations>({});
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [hubbleByCluster, setHubbleByCluster] = useState<Record<string, string>>({});
  const [podMap, setPodMap] = useState<Record<string, ClusterPodSummary[]>>({});
  const [podLoading, setPodLoading] = useState<Record<string, boolean>>({});
  const [expandedApps, setExpandedApps] = useState<Set<string>>(new Set());
  const [tab, setTab] = useQueryParam('tab');
  const activeTab = (tab || 'overview').toLowerCase();
  const [packetwolfStatus, setPacketwolfStatus] = useState<PacketWolfStatus | null>(null);
  const [packetwolfFlows, setPacketwolfFlows] = useState<Record<string, unknown> | null>(null);
  const [edgeAgents, setEdgeAgents] = useState<EdgeAgentRecord[]>([]);
  const [placementPlan, setPlacementPlan] = useState<FederationPlan | null>(null);
  const [placementLoading, setPlacementLoading] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);

  const hubbleApps = useMemo(
    () => workloads.filter((w) => w.cluster && hubbleByCluster[w.cluster!]).slice(0, 20),
    [workloads, hubbleByCluster],
  );

  const fetchedPodsRef = useRef<Set<string>>(new Set());

  const fetchPodsForApp = useCallback(async (app: WorkloadResponse) => {
    const ref = parseClusterWorkload(app);
    if (!ref) return;
    const key = app.name;
    if (fetchedPodsRef.current.has(key)) return;
    fetchedPodsRef.current.add(key);
    setPodLoading((m) => ({ ...m, [key]: true }));
    const detail = await apiFetch<ClusterResourceDetail>(clusterResourceQuery(ref));
    setPodMap((m) => ({ ...m, [key]: detail?.pods ?? [] }));
    setPodLoading((m) => ({ ...m, [key]: false }));
  }, []);

  const toggleApp = useCallback(
    (app: WorkloadResponse) => {
      setExpandedApps((prev) => {
        const next = new Set(prev);
        if (next.has(app.name)) {
          next.delete(app.name);
        } else {
          next.add(app.name);
          void fetchPodsForApp(app);
        }
        return next;
      });
    },
    [fetchPodsForApp],
  );

  const loadAllPods = useCallback(async () => {
    await Promise.all(hubbleApps.map((app) => fetchPodsForApp(app)));
    setExpandedApps(new Set(hubbleApps.map((a) => a.name)));
  }, [hubbleApps, fetchPodsForApp]);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [clusterRes, serverRes, workloadsRes, pwStatusRes, edgeRes] = await Promise.all([
      apiFetchSettled<ClusterSummary>('/cluster/summary'),
      apiFetchSettled<ServerPayload>('/server'),
      apiFetchSettled<WorkloadResponse[]>('/workloads'),
      apiFetchSettled<PacketWolfStatus>('/ecosystem/packetwolf/status'),
      apiFetchSettled<EdgeAgentRecord[]>('/fleet/edge/agents'),
    ]);
    if (!clusterRes.ok && !serverRes.ok) {
      setLoadFailed(true);
      setSummary(null);
    } else {
      const serverIntegrations = serverRes.ok ? (serverRes.data.integrations ?? {}) : {};
      setSummary(clusterRes.ok ? clusterRes.data : null);
      setIntegrations(serverIntegrations);
      setWorkloads(workloadsRes.ok ? workloadsRes.data.filter(isK8sApplication) : []);

      const clusters = clusterRes.ok ? clusterRes.data.clusters : [];
      const hubbleMap: Record<string, string> = {};
      const globalHubble = serverIntegrations.hubble_ui_url;
      if (globalHubble) {
        for (const c of clusters) {
          hubbleMap[c.name] = globalHubble;
        }
      } else {
        await Promise.all(
          clusters.map(async (c) => {
            const res = await apiFetchSettled<{ url?: string | null }>(
              `/cluster/cilium/hubble?cluster=${encodeURIComponent(c.name)}`,
            );
            if (res.ok && res.data.url) {
              hubbleMap[c.name] = res.data.url;
            }
          }),
        );
      }
      setHubbleByCluster(hubbleMap);
    }
    setPacketwolfStatus(pwStatusRes.ok ? pwStatusRes.data : null);
    setEdgeAgents(edgeRes.ok ? edgeRes.data : []);
    if (pwStatusRes.ok && pwStatusRes.data.configured && pwStatusRes.data.reachable) {
      const flows = await apiFetch<Record<string, unknown>>('/ecosystem/packetwolf/flows/stats');
      setPacketwolfFlows(flows);
    } else {
      setPacketwolfFlows(null);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading && !summary && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Fleet overview unavailable" onRetry={() => void load()} />;
  }

  const clusters = summary?.clusters ?? [];

  return (
    <div>
      <div className="mb-6 glass-context-banner" data-testid="fleet-hub-context">
        Fleet
        {' · '}
        <Link to={viewToPath('health')} className="text-aether hover:underline" data-testid="fleet-context-orchestrator-link">
          Orchestrator →
        </Link>
        {' · '}
        <Link to={viewToPath('intelligence')} className="text-aether hover:underline" data-testid="fleet-context-intelligence-link">
          Intelligence →
        </Link>
        {' · '}
        <Link
          to={`${viewToPath('fleet')}?tab=edge`}
          className="text-aether hover:underline"
          data-testid="fleet-context-edge-link"
        >
          Edge →
        </Link>
      </div>
      <FleetIntelligenceBrief onNavigate={(view) => { navigate(viewToPath(view)); }} />
      <MultiCloudPanel />
      <FederationPlatformPanel />
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />
      {focusedWorkload ? (
        <WorkloadContextBanner testId="fleet-workload-context" workload={focusedWorkload} description="Fleet context">
          <WorkloadScopedCrossLinks
            workload={focusedWorkload}
            prefix="fleet"
            showDrift
            showAudit
            showGitops
            showMetrics
          />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('clusters'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="fleet-clusters-scoped-link"
          >
            Clusters →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="fleet-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="fleet-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="fleet-context-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="fleet-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('copilot'), { workload: focusedWorkload, q: `Fleet guidance for ${focusedWorkload}` })}
            className="text-aether hover:underline"
            data-testid="fleet-context-copilot-link"
          >
            Copilot →
          </Link>
          {' · '}
          <Link
            to={viewToPath('hosted')}
            className="text-aether hover:underline"
            data-testid="fleet-context-hosted-link"
          >
            Hosted SaaS →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      <div className="mb-4">
        <button
          type="button"
          data-testid="fleet-clusters-link"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('clusters'), { workload: focusedWorkload })
                : viewToPath('clusters'),
            )
          }
          className="text-xs text-aether hover:underline"
        >
          Cluster browser →
        </button>
        <button
          type="button"
          data-testid="fleet-metrics-link"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('metrics'), { workload: focusedWorkload })
                : viewToPath('metrics'),
            )
          }
          className="text-xs text-aether hover:underline ml-3"
        >
          Metrics &amp; chargeback →
        </button>
        <button
          type="button"
          data-testid="fleet-scheduler-link"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('scheduler'), { workload: focusedWorkload })
                : viewToPath('scheduler'),
            )
          }
          className="text-xs text-aether hover:underline ml-3"
        >
          Placement scheduler →
        </button>
        <button
          type="button"
          data-testid="fleet-drift-link"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('drift'), { workload: focusedWorkload })
                : viewToPath('drift'),
            )
          }
          className="text-xs text-aether hover:underline ml-3"
        >
          Drift detection →
        </button>
        <button
          type="button"
          data-testid="fleet-confidential-link"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('confidential'), { workload: focusedWorkload })
                : viewToPath('confidential'),
            )
          }
          className="text-xs text-aether hover:underline ml-3"
        >
          Confidential fleet →
        </button>
        <button
          type="button"
          data-testid="fleet-health-link"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('health'), { workload: focusedWorkload })
                : viewToPath('health'),
            )
          }
          className="text-xs text-aether hover:underline ml-3"
        >
          Health monitor →
        </button>
        <button
          type="button"
          data-testid="fleet-events-link"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('events'), { workload: focusedWorkload })
                : viewToPath('events'),
            )
          }
          className="text-xs text-aether hover:underline ml-3"
        >
          Events feed →
        </button>
      </div>

      <div className="flex flex-wrap gap-2 mb-6" data-testid="fleet-tabs">
        {(['overview', 'edge', 'placement'] as const).map((t) => (
          <button
            key={t}
            type="button"
            data-testid={t === 'edge' ? 'fleet-edge-tab' : t === 'placement' ? 'fleet-placement-tab' : 'fleet-overview-tab'}
            onClick={() => setTab(t === 'overview' ? '' : t)}
            className={`glass-tab tab-chip capitalize ${activeTab === t ? 'glass-tab-active tab-chip-active' : ''}`}
          >
            {t === 'edge' ? 'Edge Sites' : t === 'placement' ? 'Placement' : 'Overview'}
          </button>
        ))}
      </div>

      {activeTab === 'edge' ? (
        <div className="glass-panel-card mb-6" data-testid="fleet-edge-panel">
          <h2 className="text-lg font-semibold text-slate-100 mb-4">Edge sites</h2>
          {edgeAgents.length === 0 ? (
            <p className="text-sm text-slate-500">No edge agents registered. Run <code className="text-slate-400">aether edge-agent</code> at remote sites.</p>
          ) : (
            <div className="space-y-3">
              {edgeAgents.map((a) => (
                <div key={a.site} className="rounded-lg border glass-divider p-3 flex items-center justify-between gap-3">
                  <div>
                    <div className="text-sm font-medium text-slate-100">{a.site}</div>
                    <div className="text-xs text-slate-500">{a.kube_context ?? 'default context'} · queue depth {a.queue_depth}</div>
                  </div>
                  <Badge text={a.online ? 'online' : 'stale'} variant={a.online ? 'green' : 'yellow'} />
                </div>
              ))}
            </div>
          )}
        </div>
      ) : null}

      {activeTab === 'placement' ? (
        <div className="glass-panel-card mb-6" data-testid="fleet-placement-panel">
          <div className="flex items-center justify-between gap-3 mb-4">
            <h2 className="text-lg font-semibold text-slate-100">Federation placement</h2>
            <button
              type="button"
              data-testid="fleet-placement-run"
              disabled={placementLoading || !focusedWorkload}
              onClick={async () => {
                if (!focusedWorkload) return;
                setPlacementLoading(true);
                const res = await apiPost<FederationPlan>('/fleet/federation/plan', { workload_name: focusedWorkload });
                setPlacementPlan(res.success ? (res.data as FederationPlan) : null);
                setPlacementLoading(false);
              }}
              className="rounded-lg bg-aether px-3 py-1.5 text-sm text-white disabled:opacity-50"
            >
              {placementLoading ? 'Planning…' : 'Plan for focused workload'}
            </button>
          </div>
          {!focusedWorkload ? (
            <p className="text-sm text-slate-500">Add <code className="text-slate-400">?workload=name</code> to plan federation placement.</p>
          ) : placementPlan ? (
            <div>
              {placementPlan.anomaly_signals_configured ? (
                <p className="text-xs text-cyan-300 mb-2" data-testid="placement-anomaly-hint">
                  PacketWolf anomalies considered ({placementPlan.total_anomalies ?? 0} signals)
                  {placementPlan.recommended_cluster ? ` · recommended: ${placementPlan.recommended_cluster}` : ''}
                </p>
              ) : null}
              <CardGrid columns="compact">
                {placementPlan.clusters.map((c, i) => (
                  <EntityCard
                    key={c.cluster}
                    index={i}
                    icon={<Server size={18} />}
                    statusTone={c.reachable ? 'green' : 'red'}
                    pulse={c.reachable && c.cluster === placementPlan.recommended_cluster}
                    title={c.cluster}
                    subtitle={c.runtime_hint}
                    badge={<Badge text={c.reachable ? 'reachable' : 'down'} variant={c.reachable ? 'green' : 'red'} />}
                    body={
                      <div className="flex flex-wrap gap-1.5">
                        <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-slate-300">
                          Score {c.score.toFixed(1)}
                        </span>
                        <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-slate-300">
                          {c.anomaly_count ?? 0} anomalies
                        </span>
                      </div>
                    }
                  />
                ))}
              </CardGrid>
            </div>
          ) : (
            <p className="text-sm text-slate-500">Run placement plan to rank clusters for {focusedWorkload}.</p>
          )}
        </div>
      ) : null}

      {activeTab === 'overview' ? (
      <>
      <div className="overview-section-shell mb-8 grid grid-cols-1 gap-3 p-6 sm:grid-cols-2 lg:grid-cols-4 sm:p-8">
        <StatCard title="Clusters" value={summary?.cluster_count ?? 0} color="blue" icon={<Globe size={16} />} compact isEmpty={(summary?.cluster_count ?? 0) === 0} />
        <button type="button" onClick={() => navigate(viewToPath('health'))} className="text-left" data-testid="fleet-healthy-stat">
          <StatCard title="Healthy" value={summary?.healthy_clusters ?? 0} color="green" icon={<Shield size={16} />} compact isEmpty={(summary?.healthy_clusters ?? 0) === 0} />
        </button>
        <button
          type="button"
          data-testid="fleet-workloads-stat"
          onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { source: 'cluster' }))}
          className="text-left"
        >
          <StatCard title="Workloads" value={summary?.workload_count ?? 0} color="purple" icon={<Server size={16} />} compact isEmpty={(summary?.workload_count ?? 0) === 0} />
        </button>
        <StatCard
          title="Backend"
          value={summary?.connected ? 'connected' : 'offline'}
          color={summary?.connected ? 'green' : 'red'}
          icon={<Network size={16} />}
          compact
        />
      </div>

      {summary?.error && (
        <div className="glass-alert-warn mb-4 text-sm text-amber-200">
          Kubeconfig inventory: {summary.error}
        </div>
      )}

      <div className="glass-panel-card mb-6">
        <h2 className="panel-title mb-4">Registered clusters</h2>
        {clusters.length === 0 ? (
          <p className="text-sm text-slate-500">
            No clusters in kubeconfig inventory. Configure kubeconfig on the API server or open{' '}
            <button type="button" onClick={() => navigate(viewToPath('clusters'))} className="text-aether hover:underline">
              Cluster Browser
            </button>
            .
          </p>
        ) : (
          <div className="space-y-3">
            <CardGrid columns="compact">
              {clusters.map((c, i) => (
                <EntityCard
                  key={c.name}
                  index={i}
                  testId={`fleet-cluster-${c.name}`}
                  icon={<Server size={18} />}
                  statusTone={c.reachable ? 'green' : 'red'}
                  pulse={c.reachable}
                  title={c.name}
                  subtitle={c.version ? `Kubernetes ${c.version}` : 'Cluster'}
                  badge={<Badge text={c.reachable ? 'reachable' : 'unreachable'} variant={c.reachable ? 'green' : 'red'} />}
                  onClick={() => navigate(pathWithQuery(viewToPath('clusters'), { cluster: c.name }))}
                  body={
                    <code className="block truncate rounded-lg glass-inset-surface px-2.5 py-1.5 font-mono text-[11px] text-slate-400" title={c.server ?? undefined}>
                      {c.server ?? '—'}
                    </code>
                  }
                />
              ))}
            </CardGrid>
          </div>
        )}
        <p className="mt-4 text-xs text-slate-500">
          Backend: {summary?.backend ?? '—'}
          {summary?.summary_note ? ` · ${summary.summary_note}` : ''}
        </p>
      </div>

      <div className="glass-panel-card mb-6">
        <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="panel-title">Hubble flow links</h2>
            <p className="text-sm text-slate-500 mt-1">
              Per-pod deep links into Hubble UI. Expand an application to see individual pod flows.
            </p>
          </div>
          {hubbleApps.length > 0 && (
            <button
              type="button"
              onClick={() => void loadAllPods()}
              className="rounded-xl border border-purple-500/30 bg-purple-950/20 px-3 py-1.5 text-xs text-purple-200 hover:bg-purple-950/40"
            >
              Expand all &amp; load pods
            </button>
          )}
        </div>
        {hubbleApps.length === 0 ? (
          <p className="text-sm text-slate-500">No Kubernetes applications with Hubble URLs discovered.</p>
        ) : (
          <div className="space-y-2">
            {hubbleApps.map((app) => {
              const base = app.cluster ? hubbleByCluster[app.cluster] : null;
              const ns = app.namespace ?? 'default';
              const shortName = app.name.split('/').pop() ?? app.name;
              const expanded = expandedApps.has(app.name);
              const pods = podMap[app.name] ?? [];
              const loadingPods = podLoading[app.name];
              const nsHref = base ? hubbleNamespaceUrl(base, ns) : null;

              return (
                <div key={app.name} className="rounded-xl border glass-divider overflow-hidden">
                  <button
                    type="button"
                    onClick={() => toggleApp(app)}
                    className="w-full flex items-center gap-3 px-4 py-3 text-left glass-inset-hover"
                  >
                    {expanded ? (
                      <ChevronDown size={16} className="text-slate-500 shrink-0" />
                    ) : (
                      <ChevronRight size={16} className="text-slate-500 shrink-0" />
                    )}
                    <span className="font-medium text-slate-100 flex-1">{shortName}</span>
                    <span className="text-xs text-slate-500">{app.cluster} · {ns}</span>
                    {nsHref && (
                      <a
                        href={nsHref}
                        target="_blank"
                        rel="noreferrer"
                        onClick={(e) => e.stopPropagation()}
                        className="inline-flex items-center gap-1 text-purple-300 hover:text-purple-200 text-xs shrink-0"
                      >
                        Namespace <ExternalLink size={12} />
                      </a>
                    )}
                  </button>
                  {expanded && (
                    <div className="glass-divider-t px-4 py-3 glass-panel-card">
                      {loadingPods ? (
                        <p className="text-xs text-slate-500">Loading pods…</p>
                      ) : pods.length === 0 ? (
                        <p className="text-xs text-slate-500">No pods found for this application.</p>
                      ) : (
                        <div className="space-y-2">
                          {pods.map((pod) => (
                            <div
                              key={pod.name}
                              className="flex flex-wrap items-center justify-between gap-2 rounded-lg border glass-divider px-3 py-2"
                            >
                              <div className="min-w-0">
                                <span className="text-sm font-mono text-slate-200">{pod.name}</span>
                                <span className="text-xs text-slate-500 ml-2">
                                  {pod.phase} · {pod.ready}/{pod.total_containers} ready
                                  {pod.restarts > 0 ? ` · ${pod.restarts} restarts` : ''}
                                </span>
                              </div>
                              {base && (
                                <a
                                  href={hubbleWorkloadUrl(base, ns, pod.name)}
                                  target="_blank"
                                  rel="noreferrer"
                                  className="inline-flex items-center gap-1 text-purple-300 hover:text-purple-200 text-xs"
                                  data-testid={`hubble-pod-${pod.name}`}
                                >
                                  Pod flows <ExternalLink size={12} />
                                </a>
                              )}
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="glass-panel-card">
        <h2 className="panel-title mb-4">Network observability</h2>
        <p className="text-sm text-slate-500 mb-4">
          Deep Hubble flow queries and PacketWolf east-west verification are integrated via env URLs on the control plane.
        </p>
        {packetwolfStatus?.configured ? (
          <div className="mb-4 rounded-xl border border-cyan-800/40 bg-cyan-950/20 px-4 py-3 text-sm" data-testid="packetwolf-live-card">
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-medium text-cyan-200">PacketWolf bridge</span>
              <Badge text={packetwolfStatus.reachable ? 'live' : 'offline'} variant={packetwolfStatus.reachable ? 'green' : 'yellow'} />
              {packetwolfStatus.version ? <span className="text-xs text-slate-400">v{packetwolfStatus.version}</span> : null}
            </div>
            {packetwolfFlows ? (
              <pre className="mt-2 text-xs text-slate-400 overflow-auto">{JSON.stringify(packetwolfFlows, null, 2)}</pre>
            ) : null}
            {packetwolfStatus.hint ? <p className="mt-2 text-xs text-amber-300">{packetwolfStatus.hint}</p> : null}
          </div>
        ) : null}
        <div className="flex flex-wrap gap-3">
          {integrations.grafana_url ? (
            <a
              href={integrations.grafana_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-aether/40 bg-aether/10 px-4 py-2 text-sm text-aether hover:bg-aether/20"
            >
              Grafana <ExternalLink size={14} />
            </a>
          ) : null}
          {integrations.prometheus_url ? (
            <a
              href={integrations.prometheus_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border glass-divider px-4 py-2 text-sm text-slate-200 glass-inset-hover"
            >
              Prometheus <ExternalLink size={14} />
            </a>
          ) : null}
          {integrations.hubble_ui_url ? (
            <a
              href={integrations.hubble_ui_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-purple-700/50 bg-purple-950/30 px-4 py-2 text-sm text-purple-200 hover:bg-purple-950/50"
            >
              Hubble UI <ExternalLink size={14} />
            </a>
          ) : (
            <span className="text-sm text-slate-500 self-center">
              Set <code className="text-slate-400">AETHER_HUBBLE_UI_URL</code> for Hubble
            </span>
          )}
          {integrations.packetwolf_url ? (
            <a
              href={integrations.packetwolf_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-cyan-700/50 bg-cyan-950/30 px-4 py-2 text-sm text-cyan-200 hover:bg-cyan-950/50"
            >
              PacketWolf <ExternalLink size={14} />
            </a>
          ) : (
            <span className="text-sm text-slate-500 self-center">
              Set <code className="text-slate-400">AETHER_PACKETWOLF_URL</code> for PacketWolf
            </span>
          )}
          <button
            type="button"
            onClick={() => navigate(viewToPath('platform'))}
            className="inline-flex items-center gap-2 rounded-xl border glass-divider px-4 py-2 text-sm text-slate-300 glass-inset-hover"
          >
            Platform & HA settings
          </button>
        </div>
      </div>
      </>
      ) : null}
    </div>
  );
}

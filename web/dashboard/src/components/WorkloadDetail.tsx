// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useRef } from 'react';
import { Link } from 'react-router';
import { Link2 } from 'lucide-react';
import { apiFetch, apiPost, apiDelete, apiWebSocketUrl } from '../utils/api';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import LogViewer from './LogViewer';
import Badge, { RuntimeBadge, SeverityBadge } from './Badge';
import BarChart from './BarChart';
import RadarChart from './RadarChart';
import IntentDebugger from './IntentDebugger';
import ConfidentialWorkloadPanel from './ConfidentialWorkloadPanel';
import type { ClusterResourceDetail, Event, ScoringResult, WorkloadResponse } from '../types/api';

export type DetailTab = 'overview' | 'logs' | 'manifest' | 'drift' | 'scoring' | 'events' | 'trust';

interface WorkloadDetailProps {
  workload: WorkloadResponse;
  onClose: () => void;
  onAction: () => void;
  onMigrate?: (name: string) => void;
  initialTab?: DetailTab;
  canMutate?: boolean;
}

function getStatusVariant(status: string): 'green' | 'red' | 'yellow' | 'muted' {
  const s = status.toLowerCase();
  if (s === 'running' || s === 'healthy') return 'green';
  if (s === 'error' || s === 'failed') return 'red';
  if (s === 'stopped' || s === 'exited') return 'muted';
  return 'yellow';
}

function ScoringResultsView({ data }: { data: ScoringResult }) {
  const rec = data.scores.find((s) => s.runtime === data.recommended);
  const radarDims = rec
    ? [
        { label: 'Cost', value: Math.min(rec.cost_score, 1) },
        { label: 'Perf', value: Math.min(rec.performance_score, 1) },
        { label: 'Reliability', value: Math.min(rec.reliability_score, 1) },
        { label: 'Availability', value: Math.min(rec.availability_score, 1) },
      ]
    : [];

  return (
    <div className="mt-4 grid grid-cols-1 md:grid-cols-2 gap-4">
      <div>
        <span className="text-zinc-500 text-xs">RECOMMENDED RUNTIME</span>
        <div className="mt-1 flex items-center gap-2">
          <RuntimeBadge runtime={data.recommended} />
          <span className="text-lg font-semibold text-aether">{data.recommended}</span>
        </div>
        <div className="mt-3">
          <span className="text-zinc-500 text-xs">WORKLOAD CLASS</span>
          <p className="text-white">{data.workload_class}</p>
        </div>
        <div className="mt-3">
          <span className="text-zinc-500 text-xs">CONFIDENCE</span>
          <BarChart label="Confidence" percent={data.confidence * 100} />
        </div>
        {rec && (
          <div className="mt-4 space-y-2">
            <BarChart label="Cost" percent={rec.cost_score * 100} />
            <BarChart label="Performance" percent={rec.performance_score * 100} />
            <BarChart label="Reliability" percent={rec.reliability_score * 100} />
            <BarChart label="Availability" percent={rec.availability_score * 100} />
          </div>
        )}
      </div>
      <div className="flex flex-col items-center">
        {radarDims.length >= 3 && <RadarChart dimensions={radarDims} />}
        <div className="w-full mt-3 space-y-2">
          {data.scores.map((s) => (
            <div
              key={s.runtime}
              className={`rounded-lg border px-3 py-2 ${
                s.runtime === data.recommended
                  ? 'border-aether/30 bg-aether/5'
                  : 'border-zinc-700 bg-zinc-800/50'
              }`}
            >
              <div className="flex items-center justify-between gap-2 mb-1">
                <RuntimeBadge runtime={s.runtime} />
                <span className={`text-sm font-medium ${s.runtime === data.recommended ? 'text-aether' : 'text-zinc-300'}`}>
                  {(s.total_score * 100).toFixed(0)}%
                </span>
              </div>
              <BarChart label="Total score" percent={s.total_score * 100} />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export default function WorkloadDetail({ workload, onClose, onAction, onMigrate, initialTab = 'overview', canMutate = true }: WorkloadDetailProps) {
  const isAetherManaged = (workload.source ?? 'aether') === 'aether';
  const isKubeWorkload = workload.runtime.toLowerCase().includes('kube') || (!isAetherManaged && Boolean(workload.cluster));
  const networkPolicyName = `${workload.name}-netpol`;
  const ciliumPolicyName = `${workload.name}-cilium`;
  const clusterBrowseNetworkPath = pathWithQuery(viewToPath('clusters'), {
    cluster: workload.cluster ?? undefined,
    namespace: workload.namespace ?? undefined,
    kind: 'CiliumNetworkPolicy',
    tab: 'network',
  });
  const isScalableClusterWorkload = !isAetherManaged && ['Deployment', 'StatefulSet'].includes(workload.kind ?? '');
  const clusterResourceName = workload.name.split('/').pop() ?? workload.name;
  const clusterLogsPath = !isAetherManaged && workload.cluster && workload.namespace && workload.kind
    ? `/cluster/logs?cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&kind=${encodeURIComponent(workload.kind)}&name=${encodeURIComponent(clusterResourceName)}`
    : undefined;
  const clusterResourcePath = !isAetherManaged && workload.cluster && workload.namespace && workload.kind
    ? `/cluster/resource?cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&kind=${encodeURIComponent(workload.kind)}&name=${encodeURIComponent(clusterResourceName)}`
    : undefined;
  const [activeTab, setActiveTab] = useState<DetailTab>(initialTab);
  const [events, setEvents] = useState<Event[]>([]);
  const [eventsLoading, setEventsLoading] = useState(false);
  const [shellOpen, setShellOpen] = useState(false);
  const [shellOutput, setShellOutput] = useState('');
  const [shellInput, setShellInput] = useState('');
  const [shellConnected, setShellConnected] = useState(false);
  const [shellFullscreen, setShellFullscreen] = useState(false);
  const shellSocketRef = useRef<WebSocket | null>(null);
  const [driftData, setDriftData] = useState<Record<string, unknown> | null>(null);
  const [scoringData, setScoringData] = useState<ScoringResult | null>(null);
  const [scoringError, setScoringError] = useState('');
  const [clusterDetail, setClusterDetail] = useState<ClusterResourceDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState('');
  const [linkCopied, setLinkCopied] = useState(false);
  const [snapshots, setSnapshots] = useState<Array<{ version: number; path: string }>>([]);
  const [snapshotsLoading, setSnapshotsLoading] = useState(false);
  const [buildResult, setBuildResult] = useState<string | null>(null);

  async function copyShareLink() {
    const params: Record<string, string> = { workload: workload.name };
    if (activeTab !== 'overview') params.tab = activeTab;
    const url = `${window.location.origin}${pathWithQuery(viewToPath('workloads'), params)}`;
    try {
      await navigator.clipboard.writeText(url);
      setLinkCopied(true);
      window.setTimeout(() => setLinkCopied(false), 2000);
    } catch {
      /* clipboard unavailable */
    }
  }
  const [replicasInput, setReplicasInput] = useState('1');

  useEffect(() => {
    setActiveTab(initialTab);
  }, [initialTab, workload.name]);

  useEffect(() => {
    if (activeTab !== 'overview' || !isAetherManaged) return;
    setSnapshotsLoading(true);
    void apiFetch<Array<{ version: number; path: string }>>(`/workloads/${workload.name}/snapshots`).then((rows) => {
      setSnapshots(rows ?? []);
      setSnapshotsLoading(false);
    });
  }, [activeTab, isAetherManaged, workload.name]);

  async function rollbackSnapshot(version: number) {
    setActionLoading(`rollback-v${version}`);
    try {
      await apiPost(`/workloads/${workload.name}/rollback`, { version });
      onAction();
      const rows = await apiFetch<Array<{ version: number; path: string }>>(`/workloads/${workload.name}/snapshots`);
      setSnapshots(rows ?? []);
    } finally {
      setActionLoading('');
    }
  }

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return;
      const shortcuts: Record<string, DetailTab> = {
        '1': 'overview',
        '2': 'logs',
        '3': 'manifest',
        '4': 'drift',
        '5': 'scoring',
        '6': 'events',
        l: 'logs',
        m: 'manifest',
        d: 'drift',
      };
      const tab = shortcuts[e.key];
      if (tab) {
        e.preventDefault();
        setActiveTab(tab);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  useEffect(() => {
    if (activeTab === 'drift' && isAetherManaged) {
      apiFetch<Record<string, unknown>>(`/drift/${workload.name}`).then(r => {
        if (r) setDriftData(r);
      });
    }
  }, [activeTab, isAetherManaged, workload.name]);

  useEffect(() => {
    if (activeTab !== 'events') return;
    setEventsLoading(true);
    apiFetch<Event[]>('/events').then((data) => {
      setEvents((data ?? []).filter((ev) => ev.workload === workload.name));
      setEventsLoading(false);
    });
  }, [activeTab, workload.name]);

  // Shell WebSocket Connection
  useEffect(() => {
    if (!shellOpen || !workload.cluster || !workload.namespace) return;

    const wsUrl = apiWebSocketUrl(
      `/cluster/ws/exec?cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&pod=${encodeURIComponent(workload.name)}&command=/bin/sh`
    );

    try {
      const socket = new WebSocket(wsUrl);
      shellSocketRef.current = socket;

      socket.onopen = () => {
        setShellConnected(true);
        setShellOutput(prev => prev + '[aether] Connected to pod shell\n');
      };

      socket.onmessage = (event) => {
        setShellOutput(prev => prev + event.data);
      };

      socket.onclose = () => {
        setShellConnected(false);
        setShellOutput(prev => prev + '\n[aether] Connection closed\n');
      };

      socket.onerror = () => {
        setShellOutput(prev => prev + '\n[aether] Connection error\n');
      };
    } catch (e) {
      setShellOutput('[aether] Failed to connect to shell\n');
    }

    return () => {
      shellSocketRef.current?.close();
    };
  }, [shellOpen, workload.name, workload.cluster, workload.namespace]);

  useEffect(() => {
    if (activeTab === 'manifest' && clusterResourcePath && !clusterDetail) {
      apiFetch<ClusterResourceDetail>(clusterResourcePath).then((detail) => {
        if (detail) {
          setClusterDetail(detail);
          const replicas = (detail.manifest?.spec as { replicas?: number } | undefined)?.replicas;
          if (typeof replicas === 'number') {
            setReplicasInput(String(replicas));
          }
        }
      });
    }
  }, [activeTab, clusterDetail, clusterResourcePath]);

  const handleAction = async (action: string, replicas?: number) => {
    setActionLoading(action);
    try {
      if (isAetherManaged) {
        if (action === 'stop') await apiPost(`/workloads/${workload.name}/stop`);
        else if (action === 'start') await apiPost(`/workloads/${workload.name}/start`);
        else if (action === 'restart') {
          await apiPost(`/workloads/${workload.name}/restart`);
        }         else if (action === 'rollback') {
          await apiPost(`/workloads/${workload.name}/rollback`, {});
        } else if (action === 'build') {
          const res = await apiPost<{ full_name?: string; message?: string }>(`/workloads/${workload.name}/build`);
          if (res.success) {
            setBuildResult(res.data?.full_name ?? res.data?.message ?? 'Build completed');
          } else {
            setBuildResult(res.error ?? 'Build failed');
          }
        } else if (action === 'delete') {
          await apiDelete(`/workloads/${workload.name}`, { label: `Delete workload "${workload.name}"` });
          onClose();
        }
      } else if (workload.cluster && workload.namespace && workload.kind) {
        const replicas = action === 'scale' ? Number.parseInt(replicasInput, 10) : undefined;
        const response = await apiPost<string>('/cluster/action', {
          cluster: workload.cluster,
          namespace: workload.namespace,
          kind: workload.kind,
          name: clusterResourceName,
          action,
          replicas: Number.isFinite(replicas) ? replicas : undefined,
        });
        if (!response.success) {
          throw new Error(response.error ?? `cluster action ${action} failed`);
        }
        if (action === 'delete') {
          onClose();
        }
      }
      onAction();
      if (!isAetherManaged && clusterResourcePath) {
        const detail = await apiFetch<ClusterResourceDetail>(clusterResourcePath);
        if (detail) {
          setClusterDetail(detail);
          const replicas = (detail.manifest?.spec as { replicas?: number } | undefined)?.replicas;
          if (typeof replicas === 'number') {
            setReplicasInput(String(replicas));
          }
        }
      }
    } catch (e) {
      console.error(`Action ${action} failed:`, e);
    } finally {
      setActionLoading('');
    }
  };

  const tabs: { id: DetailTab; label: string }[] = [
    { id: 'overview', label: 'Overview' },
    { id: 'logs', label: 'Logs' },
    { id: 'manifest', label: 'Manifest' },
    { id: 'drift', label: 'Drift' },
    { id: 'scoring', label: 'Scoring' },
    { id: 'trust', label: 'Trust' },
    { id: 'events', label: 'Events' },
  ];

  return (
    <>
    <div className="bg-zinc-900 border border-zinc-700 rounded-xl mt-4 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-zinc-800 border-b border-zinc-700">
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-bold text-white">{workload.name}</h3>
          <Badge text={workload.status} variant={getStatusVariant(workload.status)} />
          <span className="text-sm text-zinc-400">{workload.runtime}</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => void copyShareLink()}
            className="inline-flex items-center gap-1.5 rounded-lg border border-zinc-600 px-2.5 py-1 text-xs text-zinc-300 hover:border-aether/40 hover:text-aether transition-colors"
            title="Copy shareable link"
          >
            <Link2 className="h-3.5 w-3.5" />
            {linkCopied ? 'Copied' : 'Share link'}
          </button>
          <button type="button" onClick={onClose} className="text-zinc-400 hover:text-white text-xl leading-none" aria-label="Close">&times;</button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-zinc-700">
        {tabs.map(tab => (
          <button
            key={tab.id}
            type="button"
            data-testid={`workload-tab-${tab.id}`}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2 text-sm font-medium transition-colors ${
              activeTab === tab.id
                ? 'text-aether border-b-2 border-aether bg-zinc-800/50'
                : 'text-zinc-400 hover:text-zinc-200'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="p-4">
        {activeTab === 'overview' && (
          <div>
            {/* Action Buttons */}
            {isAetherManaged && canMutate ? (
              <div className="flex gap-2 mb-4 flex-wrap">
                <button
                  onClick={() => setShellOpen(true)}
                  className="px-3 py-1.5 text-sm font-medium rounded bg-emerald-600/20 text-emerald-400 hover:bg-emerald-600/40 border border-emerald-600/30 transition-colors"
                >
                  Shell
                </button>
                {['start', 'stop', 'restart', 'build', 'rollback', 'delete'].map(action => (
                  <button
                    key={action}
                    onClick={() => handleAction(action)}
                    disabled={!!actionLoading}
                    className={`px-3 py-1.5 text-sm font-medium rounded transition-colors ${
                      action === 'delete'
                        ? 'bg-red-600/20 text-red-400 hover:bg-red-600/40 border border-red-600/30'
                        : action === 'rollback'
                          ? 'bg-amber-600/20 text-amber-300 hover:bg-amber-600/40 border border-amber-600/30'
                        : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600 border border-zinc-600'
                    } disabled:opacity-50`}
                  >
                    {actionLoading === action ? '...' : action.charAt(0).toUpperCase() + action.slice(1)}
                  </button>
                ))}
                {onMigrate ? (
                  <button
                    type="button"
                    onClick={() => onMigrate(workload.name)}
                    className="px-3 py-1.5 text-sm font-medium rounded bg-aether/15 text-aether hover:bg-aether/25 border border-aether/30 transition-colors"
                  >
                    Migrate
                  </button>
                ) : null}
                {buildResult ? (
                  <p data-testid="workload-build-result" className="w-full text-xs text-zinc-400 mt-1">
                    Build: {buildResult}
                  </p>
                ) : null}
              </div>
            ) : (
              <div className="mb-4 space-y-3">
                <div className="rounded-lg border border-blue-500/20 bg-blue-500/10 px-3 py-2 text-sm text-blue-300">
                  This resource is discovered directly from Kubernetes. Aether can inspect it and execute native cluster actions from this panel.
                </div>
                <div className="flex flex-wrap items-end gap-2">
                  <button
                    onClick={() => handleAction('restart')}
                    disabled={!!actionLoading}
                    className="px-3 py-1.5 text-sm font-medium rounded bg-zinc-700 text-zinc-300 hover:bg-zinc-600 border border-zinc-600 disabled:opacity-50"
                  >
                    {actionLoading === 'restart' ? '...' : 'Restart'}
                  </button>
                  {isScalableClusterWorkload && (
                    <>
                      <div>
                        <label className="mb-1 block text-xs text-zinc-500">Replicas</label>
                        <input
                          type="number"
                          min={0}
                          value={replicasInput}
                          onChange={(e) => setReplicasInput(e.target.value)}
                          className="w-24 rounded border border-zinc-600 bg-zinc-800 px-2 py-1.5 text-sm text-white"
                        />
                      </div>
                      <button
                        onClick={() => handleAction('scale')}
                        disabled={!!actionLoading}
                        className="px-3 py-1.5 text-sm font-medium rounded bg-blue-600/20 text-blue-300 hover:bg-blue-600/30 border border-blue-600/30 disabled:opacity-50"
                      >
                        {actionLoading === 'scale' ? '...' : 'Scale'}
                      </button>
                    </>
                  )}
                  <button
                    onClick={() => handleAction('delete')}
                    disabled={!!actionLoading}
                    className="px-3 py-1.5 text-sm font-medium rounded bg-red-600/20 text-red-400 hover:bg-red-600/40 border border-red-600/30 disabled:opacity-50"
                  >
                    {actionLoading === 'delete' ? '...' : 'Delete'}
                  </button>
                </div>
              </div>
            )}

            {/* Info Grid */}
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div><span className="text-zinc-500">Runtime</span><p className="text-white">{workload.runtime}</p></div>
              <div><span className="text-zinc-500">Image</span><p className="text-white font-mono text-xs">{workload.image}</p></div>
              <div><span className="text-zinc-500">Status</span><p className="text-white">{workload.status}</p></div>
              <div><span className="text-zinc-500">Created</span><p className="text-white">{workload.created_at?.slice(0, 19)}</p></div>
              {workload.cluster && <div><span className="text-zinc-500">Cluster</span><p className="text-white">{workload.cluster}</p></div>}
              {workload.namespace && <div><span className="text-zinc-500">Namespace</span><p className="text-white">{workload.namespace}</p></div>}
              {workload.kind && <div><span className="text-zinc-500">Kind</span><p className="text-white">{workload.kind}</p></div>}
              <div><span className="text-zinc-500">Source</span><p className="text-white capitalize">{workload.source ?? 'aether'}</p></div>
            </div>

            <div className="mt-4 flex flex-wrap gap-2" data-testid="workload-quick-links">
              {(
                [
                  { label: 'AI analysis', slug: 'ai', path: pathWithQuery(viewToPath('ai'), { workload: workload.name, tab: 'analyze' }) },
                  { label: 'Scoring', slug: 'scoring', path: pathWithQuery(viewToPath('workloads'), { workload: workload.name, tab: 'scoring' }) },
                  { label: 'Drift', slug: 'drift', path: pathWithQuery(viewToPath('drift'), { workload: workload.name }) },
                  { label: 'Audit', slug: 'audit', path: pathWithQuery(viewToPath('audit'), { workload: workload.name }) },
                  { label: 'Events', slug: 'events', path: pathWithQuery(viewToPath('events'), { workload: workload.name }) },
                  { label: 'Editor', slug: 'editor', path: pathWithQuery(viewToPath('editor'), { workload: workload.name }) },
                  { label: 'Health monitor', slug: 'health', path: pathWithQuery(viewToPath('health'), { workload: workload.name }) },
                  {
                    label: 'Ops copilot',
                    slug: 'copilot',
                    path: pathWithQuery(viewToPath('copilot'), {
                      workload: workload.name,
                      q: `Why is ${workload.name} unhealthy?`,
                    }),
                  },
                  { label: 'Trust', slug: 'trust', path: pathWithQuery(viewToPath('workloads'), { workload: workload.name, tab: 'trust' }) },
                  { label: 'Confidential', slug: 'confidential', path: pathWithQuery(viewToPath('confidential'), { workload: workload.name }) },
                  { label: 'GitOps', slug: 'gitops', path: pathWithQuery(viewToPath('gitops'), { workload: workload.name }) },
                  { label: 'Backups', slug: 'backups', path: pathWithQuery(viewToPath('backups'), { workload: workload.name }) },
                  { label: 'Secrets', slug: 'secrets', path: pathWithQuery(viewToPath('secrets'), { workload: workload.name }) },
                  { label: 'Policy', slug: 'policy', path: pathWithQuery(viewToPath('policy'), { workload: workload.name }) },
                  { label: 'OpenAPI', slug: 'openapi', path: pathWithQuery(viewToPath('openapi'), { workload: workload.name }) },
                  { label: 'Platform', slug: 'platform', path: pathWithQuery(viewToPath('platform'), { workload: workload.name }) },
                  { label: 'RBAC', slug: 'rbac', path: pathWithQuery(viewToPath('rbac'), { workload: workload.name }) },
                  { label: 'Clusters', slug: 'clusters', path: pathWithQuery(viewToPath('clusters'), { workload: workload.name }) },
                  { label: 'Metrics', slug: 'metrics', path: pathWithQuery(viewToPath('metrics'), { workload: workload.name }) },
                  { label: 'SLA', slug: 'sla', path: pathWithQuery(viewToPath('sla'), { workload: workload.name }) },
                  { label: 'Deps', slug: 'deps', path: pathWithQuery(viewToPath('deps'), { workload: workload.name }) },
                  { label: 'Compose', slug: 'compose', path: pathWithQuery(viewToPath('compose'), { workload: workload.name }) },
                  { label: 'Templates', slug: 'templates', path: pathWithQuery(viewToPath('templates'), { workload: workload.name }) },
                  { label: 'Plugins', slug: 'plugins', path: pathWithQuery(viewToPath('plugins'), { workload: workload.name }) },
                  { label: 'Alerts', slug: 'alerts', path: pathWithQuery(viewToPath('alerts'), { workload: workload.name }) },
                  { label: 'Scheduler', slug: 'scheduler', path: pathWithQuery(viewToPath('scheduler'), { workload: workload.name }) },
                  { label: 'Cost', slug: 'cost', path: pathWithQuery(viewToPath('cost'), { workload: workload.name }) },
                  { label: 'Fleet', slug: 'fleet', path: pathWithQuery(viewToPath('fleet'), { workload: workload.name }) },
                  { label: 'Affinity', slug: 'affinity', path: pathWithQuery(viewToPath('affinity'), { workload: workload.name }) },
                  { label: 'Envs', slug: 'envs', path: pathWithQuery(viewToPath('envs'), { workload: workload.name }) },
                  { label: 'Intelligence', slug: 'intelligence', path: pathWithQuery(viewToPath('intelligence'), { tab: 'predictions', workload: workload.name }) },
                ] as const
              ).map((link) => (
                <a
                  key={link.label}
                  href={link.path}
                  data-testid={`workload-link-${link.slug}`}
                  className="rounded-lg border border-zinc-700 px-2.5 py-1 text-xs text-zinc-300 hover:border-aether/40 hover:text-aether transition-colors"
                >
                  {link.label}
                </a>
              ))}
            </div>

            {isAetherManaged ? (
              <div className="mt-4 rounded-lg border border-zinc-700 bg-zinc-950/60 p-3" data-testid="workload-snapshots">
                <h4 className="mb-2 text-sm font-semibold text-zinc-200">Snapshots</h4>
                {snapshotsLoading ? (
                  <p className="text-xs text-zinc-500">Loading snapshots…</p>
                ) : snapshots.length === 0 ? (
                  <p className="text-xs text-zinc-500">No snapshots yet — snapshots are created before migrations and updates.</p>
                ) : (
                  <ul className="space-y-2">
                    {snapshots.map((snap) => (
                      <li key={snap.version} className="flex flex-wrap items-center justify-between gap-2 text-sm">
                        <span className="font-mono text-xs text-zinc-400 truncate" title={snap.path}>
                          v{snap.version}
                        </span>
                        {canMutate ? (
                          <button
                            type="button"
                            onClick={() => void rollbackSnapshot(snap.version)}
                            disabled={!!actionLoading}
                            className="rounded border border-amber-600/40 px-2 py-1 text-xs text-amber-300 hover:bg-amber-600/10 disabled:opacity-50"
                          >
                            {actionLoading === `rollback-v${snap.version}` ? 'Rolling back…' : 'Rollback'}
                          </button>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            ) : null}

            {isKubeWorkload && (
              <div className="mt-4 rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Network policies</h4>
                <dl className="space-y-2 text-sm">
                  <div className="flex justify-between gap-4">
                    <dt className="text-zinc-500">NetworkPolicy</dt>
                    <dd className="font-mono text-xs text-zinc-200">{networkPolicyName}</dd>
                  </div>
                  <div className="flex justify-between gap-4">
                    <dt className="text-zinc-500">Cilium CNP</dt>
                    <dd className="font-mono text-xs text-zinc-200">{ciliumPolicyName}</dd>
                  </div>
                </dl>
                <a
                  href={clusterBrowseNetworkPath}
                  className="mt-3 inline-flex items-center gap-1.5 text-xs text-aether hover:underline"
                >
                  Browse cluster network policies
                  <Link2 className="h-3.5 w-3.5" />
                </a>
                <p className="mt-2 text-xs text-zinc-500">
                  Names follow Aether deploy conventions when <code>network.networkPolicy</code> or{' '}
                  <code>network.ciliumNetworkPolicy</code> is set in the workload spec.
                </p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'logs' && (
          <LogViewer workloadName={workload.name} logsPath={clusterLogsPath} />
        )}

        {activeTab === 'manifest' && (
          <div className="space-y-4">
            {isAetherManaged ? (
              <p className="text-zinc-500">Manifest inspection is currently available for Kubernetes resources discovered directly from the cluster.</p>
            ) : !clusterDetail ? (
              <p className="text-zinc-500">Loading Kubernetes resource details...</p>
            ) : (
              <>
                <div className="grid grid-cols-2 gap-3 text-sm">
                  <div><span className="text-zinc-500">API Version</span><p className="text-white">{clusterDetail.api_version ?? 'unknown'}</p></div>
                  <div><span className="text-zinc-500">Pods</span><p className="text-white">{clusterDetail.pods.length}</p></div>
                </div>

                {clusterDetail.conditions.length > 0 && (
                  <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                    <h4 className="mb-3 text-sm font-semibold text-zinc-200">Conditions</h4>
                    <div className="space-y-2">
                      {clusterDetail.conditions.map((condition) => (
                        <div key={`${condition.type_}:${condition.reason ?? 'none'}`} className="rounded-md bg-zinc-900 px-3 py-2 text-sm">
                          <div className="flex items-center justify-between">
                            <span className="font-medium text-zinc-100">{condition.type_}</span>
                            <span className={condition.status === 'True' ? 'text-emerald-400' : 'text-amber-400'}>
                              {condition.status}
                            </span>
                          </div>
                          {condition.reason && <div className="mt-1 text-xs text-zinc-400">{condition.reason}</div>}
                          {condition.message && <div className="mt-1 text-xs text-zinc-500">{condition.message}</div>}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {clusterDetail.pods.length > 0 && (
                  <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                    <h4 className="mb-3 text-sm font-semibold text-zinc-200">Pods</h4>
                    <div className="space-y-2">
                      {clusterDetail.pods.map((pod) => (
                        <div key={pod.name} className="grid grid-cols-5 gap-3 rounded-md bg-zinc-900 px-3 py-2 text-sm">
                          <div className="col-span-2">
                            <div className="text-zinc-100">{pod.name}</div>
                            <div className="text-xs text-zinc-500">{pod.node ?? 'node unknown'}</div>
                          </div>
                          <div className="text-zinc-300">{pod.phase}</div>
                          <div className="text-zinc-300">{pod.ready}/{pod.total_containers} ready</div>
                          <div className="text-zinc-300">{pod.restarts} restarts</div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                  <h4 className="mb-3 text-sm font-semibold text-zinc-200">Manifest</h4>
                  <pre className="max-h-96 overflow-auto rounded bg-zinc-950 p-3 text-xs text-zinc-300">
                    {JSON.stringify(clusterDetail.manifest, null, 2)}
                  </pre>
                </div>
              </>
            )}
          </div>
        )}

        {activeTab === 'drift' && (
          <div>
            {!isAetherManaged ? (
              <p className="text-zinc-500">Drift analysis is currently available only for Aether-managed workloads.</p>
            ) : driftData ? (
              <div>
                <div className={`text-sm font-medium mb-2 ${(driftData as Record<string, unknown>).has_drift ? 'text-orange-400' : 'text-emerald-400'}`}>
                  {(driftData as Record<string, unknown>).has_drift
                    ? `${((driftData as Record<string, unknown>).drifts as unknown[])?.length || 0} drift item(s) detected`
                    : 'No drift detected'}
                </div>
                {((driftData as Record<string, unknown>).drifts as Array<Record<string, string>>)?.map((d, i: number) => (
                  <div key={i} className="bg-zinc-800 rounded p-2 mb-2 text-sm">
                    <span className={`font-medium ${d.severity === 'Critical' ? 'text-red-400' : d.severity === 'Warning' ? 'text-orange-400' : 'text-blue-400'}`}>
                      [{d.severity}]
                    </span>
                    {' '}<span className="text-zinc-300">{d.field}</span>
                    <div className="text-zinc-500 mt-1">Expected: {d.expected} | Actual: {d.actual}</div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-zinc-500">Loading drift data...</p>
            )}
          </div>
        )}

        {activeTab === 'scoring' && (
          <div className="text-sm text-zinc-400">
            {!scoringData && (
              <>
                <p>Run the AI scoring engine to compare runtimes for this workload.</p>
                <button
                  type="button"
                  onClick={async () => {
                    setLoading(true);
                    setScoringError('');
                    const r = await apiPost<ScoringResult>('/ai/recommend', {});
                    if (r.success && r.data) setScoringData(r.data);
                    else setScoringError(r.error ?? 'Analysis failed');
                    setLoading(false);
                  }}
                  disabled={loading}
                  className="mt-2 px-3 py-1.5 bg-aether/20 text-aether rounded border border-aether/30 hover:bg-aether/40 disabled:opacity-50"
                >
                  {loading ? 'Analyzing...' : 'Analyze'}
                </button>
              </>
            )}
            {scoringError && <p className="mt-2 text-red-400">{scoringError}</p>}
            {scoringData && (
              <>
                <div className="flex justify-end mb-2">
                  <button
                    type="button"
                    onClick={async () => {
                      setLoading(true);
                      setScoringError('');
                      const r = await apiPost<ScoringResult>('/ai/recommend', {});
                      if (r.success && r.data) setScoringData(r.data);
                      else setScoringError(r.error ?? 'Analysis failed');
                      setLoading(false);
                    }}
                    disabled={loading}
                    className="text-xs text-aether hover:text-aether-light disabled:opacity-50"
                  >
                    {loading ? 'Analyzing...' : 'Re-analyze'}
                  </button>
                </div>
                <ScoringResultsView data={scoringData} />
              </>
            )}
            <div className="mt-6 border-t border-zinc-800 pt-4">
              <IntentDebugger />
            </div>
          </div>
        )}

        {activeTab === 'trust' && (
          <ConfidentialWorkloadPanel workloadName={workload.name} runtime={workload.runtime} />
        )}

        {activeTab === 'events' && (
          <div className="text-sm">
            <div className="mb-3 flex flex-wrap gap-3 text-xs" data-testid="workload-events-cross-links">
              <Link
                to={pathWithQuery(viewToPath('events'), { workload: workload.name })}
                className="text-aether hover:underline"
                data-testid="workload-events-events-link"
              >
                Full events feed →
              </Link>
              <Link
                to={pathWithQuery(viewToPath('alerts'), { workload: workload.name })}
                className="text-aether hover:underline"
                data-testid="workload-events-alerts-link"
              >
                Alert rules →
              </Link>
              <Link
                to={pathWithQuery(viewToPath('health'), { workload: workload.name })}
                className="text-aether hover:underline"
                data-testid="workload-events-health-link"
              >
                Health monitor →
              </Link>
              <Link
                to={pathWithQuery(viewToPath('drift'), { workload: workload.name })}
                className="text-aether hover:underline"
                data-testid="workload-events-drift-link"
              >
                Drift →
              </Link>
              <Link
                to={pathWithQuery(viewToPath('workloads'), { workload: workload.name, tab: 'trust' })}
                className="text-aether hover:underline"
                data-testid="workload-events-trust-link"
              >
                Trust &amp; attestation →
              </Link>
            </div>
            {eventsLoading ? (
              <p className="text-zinc-500">Loading events...</p>
            ) : events.length === 0 ? (
              <p className="text-zinc-500">No events recorded for this workload.</p>
            ) : (
              <div className="space-y-2 max-h-96 overflow-auto">
                {events.map((ev, i) => (
                  <div key={`${ev.timestamp}-${i}`} className="flex items-start gap-3 rounded-lg bg-zinc-950/60 p-3">
                    <SeverityBadge severity={ev.severity} />
                    <div className="flex-1 min-w-0">
                      <div className="font-medium text-zinc-200">{ev.title}</div>
                      <div className="text-xs text-zinc-500 mt-0.5">{ev.message}</div>
                      <div className="text-xs text-zinc-600 mt-1">{ev.timestamp?.slice(0, 19)}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>

      {/* Shell Modal - Real WebSocket Terminal */}
      {shellOpen && (
        <div className={`fixed inset-0 bg-black/70 flex items-center justify-center z-[60] ${shellFullscreen ? 'p-0' : ''}`}>
          <div className={`bg-zinc-950 border border-zinc-700 rounded-2xl overflow-hidden transition-all ${shellFullscreen ? 'w-full h-full max-w-none rounded-none' : 'w-full max-w-4xl mx-4'}`}>
            <div className="flex items-center justify-between px-5 py-3 border-b border-zinc-800 bg-zinc-900">
              <div className="flex items-center gap-3">
                <div className="font-medium">Shell — {workload.name}</div>
                <div className={`text-xs px-2 py-0.5 rounded ${shellConnected ? 'bg-emerald-500/20 text-emerald-400' : 'bg-zinc-700 text-zinc-400'}`}>
                  {shellConnected ? 'Connected' : 'Disconnected'}
                </div>
              </div>

              <div className="flex items-center gap-2">
                <button 
                  onClick={() => navigator.clipboard.writeText(shellOutput)}
                  className="text-xs px-3 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300"
                >
                  Copy
                </button>
                <button 
                  onClick={() => setShellOutput('')}
                  className="text-xs px-3 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300"
                >
                  Clear
                </button>
                <button 
                  onClick={() => setShellFullscreen(!shellFullscreen)}
                  className="text-xs px-3 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300"
                >
                  {shellFullscreen ? 'Exit Fullscreen' : 'Fullscreen'}
                </button>
                <button 
                  onClick={() => {
                    shellSocketRef.current?.close();
                    setShellOpen(false);
                    setShellConnected(false);
                    setShellOutput('');
                    setShellFullscreen(false);
                  }} 
                  className="text-zinc-400 hover:text-white text-xl leading-none ml-1"
                >
                  ×
                </button>
              </div>
            </div>

            <div className="p-4">
              <div 
                ref={(el) => {
                  if (el) el.scrollTop = el.scrollHeight;
                }}
                className="bg-[#0a0c10] rounded-xl p-4 font-mono text-sm text-emerald-400 h-[420px] overflow-auto whitespace-pre-wrap border border-zinc-800 shadow-inner"
              >
                {shellOutput || '[aether] Connecting to pod shell...\n'}
              </div>

              <div className="mt-4 flex gap-2 items-center">
                <select 
                  className="bg-zinc-900 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-400"
                  defaultValue="/bin/sh"
                >
                  <option value="/bin/sh">/bin/sh</option>
                  <option value="/bin/bash">/bin/bash</option>
                </select>

                <input
                  value={shellInput}
                  onChange={(e) => setShellInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && shellConnected && shellInput.trim()) {
                      shellSocketRef.current?.send(shellInput + '\n');
                      setShellOutput(prev => prev + `$ ${shellInput}\n`);
                      setShellInput('');
                    }
                  }}
                  className="flex-1 bg-zinc-900 border border-zinc-700 rounded-lg px-4 py-2.5 text-sm font-mono focus:outline-none focus:border-emerald-600"
                  placeholder="Type command and press Enter..."
                  disabled={!shellConnected}
                />
                <button
                  onClick={() => {
                    if (shellConnected && shellInput.trim()) {
                      shellSocketRef.current?.send(shellInput + '\n');
                      setShellOutput(prev => prev + `$ ${shellInput}\n`);
                      setShellInput('');
                    }
                  }}
                  disabled={!shellConnected || !shellInput.trim()}
                  className="px-6 py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors"
                >
                  Send
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

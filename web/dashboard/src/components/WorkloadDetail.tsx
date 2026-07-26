// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useRef } from 'react';
import { Link, useNavigate } from 'react-router';
import { Link2 } from 'lucide-react';
import { apiFetch, apiPost, apiDelete, apiWebSocketUrl } from '../utils/api';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import { applicationLabel, isK8sApplication, workspaceLabel } from '../utils/k8sUx';
import { pickExecPodName, isShellableClusterKind as kindSupportsShell, buildKubectlCommands, isRolloutClusterKind, canPortForwardClusterKind } from '../utils/clusterExec';
import LogViewer from './LogViewer';
import FixItPanel, { type FixAction } from './FixItPanel';
import ApplicationTopology from './ApplicationTopology';
import AiTroubleshootPanel from './AiTroubleshootPanel';
import Badge, { RuntimeBadge, SeverityBadge } from './Badge';
import BarChart from './BarChart';
import RadarChart from './RadarChart';
import IntentDebugger from './IntentDebugger';
import ConfidentialWorkloadPanel from './ConfidentialWorkloadPanel';
import type {
  ClusterHealthSummary,
  ClusterPortForwardSession,
  ClusterRelatedEvent,
  ClusterResourceDetail,
  ClusterRolloutStatus,
  ClusterTopMetric,
  Event,
  HelmRevisionEntry,
  ScoringResult,
  WorkloadResponse,
} from '../types/api';

export type DetailTab = 'overview' | 'logs' | 'manifest' | 'drift' | 'scoring' | 'events' | 'trust' | 'topology';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

interface WorkloadDetailProps {
  workload: WorkloadResponse;
  onClose: () => void;
  onAction: () => void;
  onMigrate?: (name: string) => void;
  initialTab?: DetailTab;
  /** When true, open the Shell modal as soon as the panel mounts. */
  initialShellOpen?: boolean;
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
        <span className="text-slate-500 text-xs">RECOMMENDED RUNTIME</span>
        <div className="mt-1 flex items-center gap-2">
          <RuntimeBadge runtime={data.recommended} />
          <span className="text-lg font-semibold text-aether">{data.recommended}</span>
        </div>
        <div className="mt-3">
          <span className="text-slate-500 text-xs">WORKLOAD CLASS</span>
          <p className="text-white">{data.workload_class}</p>
        </div>
        <div className="mt-3">
          <span className="text-slate-500 text-xs">CONFIDENCE</span>
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
                  : 'glass-divider glass-inset-surface'
              }`}
            >
              <div className="flex items-center justify-between gap-2 mb-1">
                <RuntimeBadge runtime={s.runtime} />
                <span className={`text-sm font-medium ${s.runtime === data.recommended ? 'text-aether' : 'text-slate-300'}`}>
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

export default function WorkloadDetail({
  workload,
  onClose,
  onAction,
  onMigrate,
  initialTab = 'overview',
  initialShellOpen = false,
  canMutate = true,
}: WorkloadDetailProps) {
  const navigate = useNavigate();
  const isAetherManaged = (workload.source ?? 'aether') === 'aether';
  const isKubeWorkload = isK8sApplication(workload);
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
  const canShellDiscovered =
    !isAetherManaged
    && canMutate
    && Boolean(workload.cluster && workload.namespace && workload.kind)
    && kindSupportsShell(workload.kind);
  const canShellManaged = isAetherManaged && canMutate;
  const showShell = canShellDiscovered || canShellManaged;
  const clusterLogsPath = !isAetherManaged && workload.cluster && workload.namespace && workload.kind
    ? `/cluster/logs?cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&kind=${encodeURIComponent(workload.kind)}&name=${encodeURIComponent(clusterResourceName)}`
    : undefined;
  const clusterResourcePath = !isAetherManaged && workload.cluster && workload.namespace && workload.kind
    ? `/cluster/resource?cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&kind=${encodeURIComponent(workload.kind)}&name=${encodeURIComponent(clusterResourceName)}`
    : undefined;
  const [activeTab, setActiveTab] = useState<DetailTab>(initialTab);
  const [events, setEvents] = useState<Event[]>([]);
  const [eventsLoading, setEventsLoading] = useState(false);
  const [shellOpen, setShellOpen] = useState(initialShellOpen);
  const [shellOutput, setShellOutput] = useState('');
  const [shellInput, setShellInput] = useState('');
  const [shellConnected, setShellConnected] = useState(false);
  const [shellFullscreen, setShellFullscreen] = useState(false);
  const [shellPod, setShellPod] = useState('');
  const [shellPods, setShellPods] = useState<Array<{ name: string; phase: string; containers: string[] }>>([]);
  const [shellContainer, setShellContainer] = useState('');
  const [shellCommand, setShellCommand] = useState('/bin/sh');
  const [copiedCmd, setCopiedCmd] = useState('');
  const [clusterEvents, setClusterEvents] = useState<ClusterRelatedEvent[]>([]);
  const [clusterMetrics, setClusterMetrics] = useState<ClusterTopMetric[]>([]);
  const [clusterHealth, setClusterHealth] = useState<ClusterHealthSummary | null>(null);
  const [rollout, setRollout] = useState<ClusterRolloutStatus | null>(null);
  const [portForwardPod, setPortForwardPod] = useState('');
  const [portForwardRemotePort, setPortForwardRemotePort] = useState('8080');
  const [portForwardLocalPort, setPortForwardLocalPort] = useState('');
  const [portForwardSession, setPortForwardSession] = useState<ClusterPortForwardSession | null>(null);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [yamlCopied, setYamlCopied] = useState(false);
  const [helmHistory, setHelmHistory] = useState<HelmRevisionEntry[]>([]);
  const [helmRevision, setHelmRevision] = useState('');
  const [opsAutoRefresh, setOpsAutoRefresh] = useState(true);
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
    if (!initialShellOpen) return;
    setShellOutput('');
    setShellPod('');
    setShellOpen(true);
  }, [initialShellOpen, workload.name]);

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
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable || target.tagName === 'SELECT') return;
      if (e.key === '?' || (e.shiftKey && e.key === '/')) {
        e.preventDefault();
        setShowShortcuts((v) => !v);
        return;
      }
      if (e.key === 's' && showShell && !shellOpen) {
        e.preventDefault();
        setShellOutput('');
        setShellPod('');
        setShellContainer('');
        setShellOpen(true);
        return;
      }
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
        e: 'events',
        o: 'overview',
      };
      const tab = shortcuts[e.key];
      if (tab) {
        e.preventDefault();
        setActiveTab(tab);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [showShell, shellOpen]);

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

  // Resolve available pods when the shell opens; reconnect when pod/container/command changes.
  useEffect(() => {
    if (!shellOpen || !workload.cluster || !workload.namespace) return;

    let cancelled = false;

    async function resolvePods() {
      let pods: Array<{ name: string; phase: string; containers: string[] }> = [];
      if (canShellDiscovered && workload.kind !== 'Pod' && clusterResourcePath) {
        const detail = await apiFetch<ClusterResourceDetail>(clusterResourcePath);
        if (cancelled) return;
        if (detail) {
          setClusterDetail(detail);
          pods = (detail.pods ?? []).map((pod) => ({
            name: pod.name,
            phase: pod.phase,
            containers: pod.containers ?? [],
          }));
        }
      } else if (canShellDiscovered && workload.kind === 'Pod' && clusterResourcePath) {
        const detail = await apiFetch<ClusterResourceDetail>(clusterResourcePath);
        if (cancelled) return;
        if (detail) {
          setClusterDetail(detail);
          const match = detail.pods.find((pod) => pod.name === clusterResourceName) ?? detail.pods[0];
          if (match) {
            pods = [{ name: match.name, phase: match.phase, containers: match.containers ?? [] }];
          } else {
            pods = [{ name: clusterResourceName, phase: 'Unknown', containers: [] }];
          }
        } else {
          pods = [{ name: clusterResourceName, phase: 'Unknown', containers: [] }];
        }
      } else {
        pods = [{ name: clusterResourceName, phase: 'Running', containers: [] }];
      }

      if (cancelled) return;
      setShellPods(pods);

      const preferred = pickExecPodName(
        workload.kind ?? undefined,
        clusterResourceName,
        pods.map((pod) => ({ name: pod.name, phase: pod.phase })),
      );
      if (!preferred) {
        setShellPod('');
        setShellOutput(
          '[aether] No pod available to exec into. For VMs, ensure the virt-launcher pod is running.\n',
        );
        return;
      }
      setShellPod((current) => {
        if (current && pods.some((pod) => pod.name === current)) return current;
        return preferred;
      });
    }

    void resolvePods();
    return () => {
      cancelled = true;
    };
  }, [
    shellOpen,
    canShellDiscovered,
    clusterResourceName,
    clusterResourcePath,
    workload.kind,
    workload.cluster,
    workload.namespace,
  ]);

  useEffect(() => {
    if (!shellOpen || !workload.cluster || !workload.namespace || !shellPod) return;

    const selected = shellPods.find((pod) => pod.name === shellPod);
    const containers = selected?.containers ?? [];
    if (containers.length > 0 && (!shellContainer || !containers.includes(shellContainer))) {
      setShellContainer(containers[0] ?? '');
      return; // reconnect after container state settles
    }

    let cancelled = false;
    setShellConnected(false);

    const params = new URLSearchParams({
      cluster: workload.cluster,
      namespace: workload.namespace,
      pod: shellPod,
      command: shellCommand || '/bin/sh',
    });
    if (shellContainer) params.set('container', shellContainer);

    const wsUrl = apiWebSocketUrl(`/cluster/ws/exec?${params.toString()}`);

    try {
      const socket = new WebSocket(wsUrl);
      shellSocketRef.current = socket;

      socket.onopen = () => {
        if (cancelled) return;
        setShellConnected(true);
        const target = shellContainer ? `${shellPod}/${shellContainer}` : shellPod;
        setShellOutput((prev) => `${prev}[aether] Connected to ${target}\n`);
      };

      socket.onmessage = (event) => {
        setShellOutput((prev) => prev + event.data);
      };

      socket.onclose = () => {
        if (cancelled) return;
        setShellConnected(false);
        setShellOutput((prev) => `${prev}\n[aether] Connection closed\n`);
      };

      socket.onerror = () => {
        if (cancelled) return;
        setShellOutput((prev) => `${prev}\n[aether] Connection error\n`);
      };
    } catch {
      setShellOutput('[aether] Failed to connect to shell\n');
    }

    return () => {
      cancelled = true;
      shellSocketRef.current?.close();
      shellSocketRef.current = null;
    };
  }, [
    shellOpen,
    shellPod,
    shellContainer,
    shellCommand,
    shellPods,
    workload.cluster,
    workload.namespace,
  ]);

  useEffect(() => {
    if ((activeTab === 'manifest' || activeTab === 'overview' || activeTab === 'logs') && clusterResourcePath && !clusterDetail) {
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

  useEffect(() => {
    if (activeTab !== 'overview' || isAetherManaged || !workload.cluster || !workload.namespace || !workload.kind) {
      return;
    }
    let cancelled = false;
    const base = `cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&kind=${encodeURIComponent(workload.kind)}&name=${encodeURIComponent(clusterResourceName)}`;

    async function loadOps() {
      const fetches: Array<Promise<unknown>> = [
        apiFetch<ClusterRelatedEvent[]>(`/cluster/events?${base}`),
        apiFetch<ClusterTopMetric[]>(`/cluster/top?${base}`),
        apiFetch<ClusterHealthSummary>(`/cluster/health?${base}`),
      ];
      if (isRolloutClusterKind(workload.kind)) {
        fetches.push(apiFetch<ClusterRolloutStatus>(`/cluster/rollout?${base}`));
      }
      const results = await Promise.all(fetches);
      if (cancelled) return;
      setClusterEvents(((results[0] as ClusterRelatedEvent[] | null) ?? []).slice(0, 8));
      setClusterMetrics((results[1] as ClusterTopMetric[] | null) ?? []);
      setClusterHealth((results[2] as ClusterHealthSummary | null) ?? null);
      if (isRolloutClusterKind(workload.kind)) {
        setRollout((results[3] as ClusterRolloutStatus | null) ?? null);
      } else {
        setRollout(null);
      }
    }

    void loadOps();
    if (!opsAutoRefresh) {
      return () => {
        cancelled = true;
      };
    }
    const interval = setInterval(() => void loadOps(), 15000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [activeTab, isAetherManaged, workload.cluster, workload.namespace, workload.kind, clusterResourceName, opsAutoRefresh]);

  // Helm revision history for discovered HelmRelease resources.
  useEffect(() => {
    if (activeTab !== 'overview' || isAetherManaged || workload.kind !== 'HelmRelease' || !workload.cluster || !workload.namespace) {
      return;
    }
    let cancelled = false;
    void apiFetch<HelmRevisionEntry[]>(
      `/cluster/helm/history?cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&release=${encodeURIComponent(clusterResourceName)}`,
    ).then((history) => {
      if (cancelled) return;
      setHelmHistory(history ?? []);
      setHelmRevision(history?.[0]?.revision ?? '');
    });
    return () => {
      cancelled = true;
    };
  }, [activeTab, isAetherManaged, workload.kind, workload.cluster, workload.namespace, clusterResourceName]);

  // Prefill port-forward target when pods load.
  useEffect(() => {
    if (!clusterDetail?.pods?.length) return;
    setPortForwardPod((current) => {
      if (current && clusterDetail.pods.some((pod) => pod.name === current)) return current;
      return pickExecPodName(
        workload.kind ?? undefined,
        clusterResourceName,
        clusterDetail.pods.map((pod) => ({ name: pod.name, phase: pod.phase })),
      );
    });
  }, [clusterDetail, clusterResourceName, workload.kind]);

  const logContainers = (() => {
    if (!clusterDetail?.pods?.length) return [] as string[];
    if (workload.kind === 'Pod') {
      return (
        clusterDetail.pods.find((pod) => pod.name === clusterResourceName)?.containers
        ?? clusterDetail.pods[0]?.containers
        ?? []
      );
    }
    const preferred = pickExecPodName(
      workload.kind ?? undefined,
      clusterResourceName,
      clusterDetail.pods.map((pod) => ({ name: pod.name, phase: pod.phase })),
    );
    return clusterDetail.pods.find((pod) => pod.name === preferred)?.containers ?? [];
  })();

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

  async function handlePortForwardStart() {
    if (!workload.cluster || !workload.namespace || !canMutate) return;
    const isService = workload.kind === 'Service';
    const target = isService ? clusterResourceName : portForwardPod;
    if (!target) return;
    setActionLoading('port-forward');
    const response = await apiPost<ClusterPortForwardSession>('/cluster/port-forward', {
      cluster: workload.cluster,
      namespace: workload.namespace,
      target_kind: isService ? 'Service' : 'Pod',
      target_name: target,
      pod: isService ? undefined : target,
      remote_port: Number.parseInt(portForwardRemotePort, 10),
      local_port: portForwardLocalPort ? Number.parseInt(portForwardLocalPort, 10) : undefined,
    });
    setActionLoading('');
    if (!response.success || !response.data) {
      toast(`Port-forward failed: ${response.error ?? 'unknown error'}`, 'error');
      return;
    }
    setPortForwardSession(response.data);
    setPortForwardLocalPort(String(response.data.local_port));
    toast(`Forwarding to ${response.data.local_url}`, 'success');
  }

  async function handlePortForwardStop() {
    if (!portForwardSession) return;
    setActionLoading('port-forward-stop');
    const response = await apiPost<string>('/cluster/port-forward/stop', {
      session_id: portForwardSession.session_id,
    });
    setActionLoading('');
    if (!response.success) {
      toast(`Stop failed: ${response.error ?? 'unknown error'}`, 'error');
      return;
    }
    setPortForwardSession(null);
    toast('Port-forward stopped', 'success');
  }

  async function handleDeletePod(podName: string) {
    if (!workload.cluster || !workload.namespace || !canMutate) return;
    setActionLoading(`delete-pod-${podName}`);
    const response = await apiPost<string>('/cluster/action', {
      cluster: workload.cluster,
      namespace: workload.namespace,
      kind: 'Pod',
      name: podName,
      action: 'delete',
    });
    setActionLoading('');
    if (!response.success) {
      toast(`Delete pod failed: ${response.error ?? 'unknown error'}`, 'error');
      return;
    }
    toast(`Pod ${podName} deleted (controller will recreate it)`, 'success');
    if (clusterResourcePath) {
      const detail = await apiFetch<ClusterResourceDetail>(clusterResourcePath);
      if (detail) setClusterDetail(detail);
    }
  }

  async function handleHelmRollback() {
    if (!workload.cluster || !workload.namespace || !canMutate || !helmRevision) return;
    setActionLoading('helm-rollback');
    const response = await apiPost<string>('/cluster/helm/action', {
      cluster: workload.cluster,
      namespace: workload.namespace,
      release: clusterResourceName,
      action: 'rollback',
      revision: helmRevision,
    });
    setActionLoading('');
    if (!response.success) {
      toast(`Helm rollback failed: ${response.error ?? 'unknown error'}`, 'error');
      return;
    }
    toast(`Rolled back ${clusterResourceName} to revision ${helmRevision}`, 'success');
    onAction();
  }

  async function handleRolloutAction(action: 'pause' | 'resume' | 'undo' | 'restart') {
    if (!workload.cluster || !workload.namespace || !workload.kind || !canMutate) return;
    setActionLoading(`rollout-${action}`);
    const response = await apiPost<string>('/cluster/rollout/action', {
      cluster: workload.cluster,
      namespace: workload.namespace,
      kind: workload.kind,
      name: clusterResourceName,
      action,
    });
    setActionLoading('');
    if (!response.success) {
      toast(`Rollout ${action} failed: ${response.error ?? 'unknown error'}`, 'error');
      return;
    }
    toast(`Rollout ${action} triggered`, 'success');
    onAction();
    const base = `cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&kind=${encodeURIComponent(workload.kind)}&name=${encodeURIComponent(clusterResourceName)}`;
    const next = await apiFetch<ClusterRolloutStatus>(`/cluster/rollout?${base}`);
    if (next) setRollout(next);
  }

  function manifestYamlText(): string {
    if (!clusterDetail?.manifest) return '';
    try {
      return JSON.stringify(clusterDetail.manifest, null, 2);
    } catch {
      return '';
    }
  }

  async function copyManifestYaml() {
    const text = manifestYamlText();
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setYamlCopied(true);
      window.setTimeout(() => setYamlCopied(false), 1500);
      toast('Manifest copied', 'success');
    } catch {
      toast('Clipboard unavailable', 'error');
    }
  }

  function downloadManifestYaml() {
    const text = manifestYamlText();
    if (!text) return;
    const blob = new Blob([text], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `${clusterResourceName}.json`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
  }

  const tabs: { id: DetailTab; label: string }[] = [
    { id: 'overview', label: 'Overview' },
    { id: 'logs', label: 'Logs' },
    ...(isKubeWorkload ? [{ id: 'topology' as const, label: 'Topology' }] : []),
    { id: 'manifest', label: 'Manifest' },
    { id: 'drift', label: 'Drift' },
    { id: 'scoring', label: 'Scoring' },
    { id: 'trust', label: 'Trust' },
    { id: 'events', label: 'Events' },
  ];

  function handleFixAction(action: FixAction) {
    switch (action) {
      case 'logs':
        setActiveTab('logs');
        break;
      case 'restart':
        void handleAction('restart');
        break;
      case 'scale':
        setActiveTab('overview');
        break;
      case 'rollback':
        void handleAction('rollback');
        break;
      case 'editor':
        navigate(pathWithQuery(viewToPath('editor'), { workload: workload.name }));
        break;
      case 'clusters':
        navigate(
          pathWithQuery(viewToPath('clusters'), {
            cluster: workload.cluster ?? undefined,
            namespace: workload.namespace ?? undefined,
          }),
        );
        break;
      case 'copilot':
        navigate(
          pathWithQuery(viewToPath('copilot'), {
            workload: workload.name,
            q: `Why is ${workload.name} failing?`,
          }),
        );
        break;
    }
  }

  return (
    <>
    <div className="glass-panel-card mt-4 overflow-hidden p-0">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 glass-divider-b glass-inset-surface">
        <div className="flex items-center gap-3">
          <div>
            <h3 className="text-lg font-bold text-white">
              {isKubeWorkload ? applicationLabel(workload) : workload.name}
            </h3>
            {isKubeWorkload && (
              <p className="text-xs text-slate-500">
                Application · {workspaceLabel(workload.namespace)}
                {workload.kind ? ` · ${workload.kind}` : ''}
              </p>
            )}
          </div>
          <Badge text={workload.status} variant={getStatusVariant(workload.status)} />
          {!isAetherManaged && clusterHealth ? (
            <Badge
              text={`health: ${clusterHealth.level}`}
              variant={
                clusterHealth.level === 'healthy' || clusterHealth.level === 'ok'
                  ? 'green'
                  : clusterHealth.level === 'warning' || clusterHealth.level === 'degraded'
                    ? 'yellow'
                    : clusterHealth.level === 'critical' || clusterHealth.level === 'unhealthy'
                      ? 'red'
                      : 'muted'
              }
            />
          ) : null}
          <span className="text-sm text-slate-400">{workload.runtime}</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setShowShortcuts((v) => !v)}
            className="btn-secondary px-2.5 py-1 text-xs"
            title="Keyboard shortcuts (?)"
            data-testid="workload-shortcuts-toggle"
          >
            ?
          </button>
          <button
            type="button"
            onClick={() => void copyShareLink()}
            className="btn-secondary inline-flex items-center gap-1.5 px-2.5 py-1 text-xs"
            title="Copy shareable link"
          >
            <Link2 className="h-3.5 w-3.5" />
            {linkCopied ? 'Copied' : 'Share link'}
          </button>
          <button type="button" onClick={onClose} className="text-slate-400 hover:text-white text-xl leading-none" aria-label="Close">&times;</button>
        </div>
      </div>

      {showShortcuts ? (
        <div className="px-4 py-2 text-xs text-slate-400 glass-divider-b glass-inset-surface" data-testid="workload-shortcuts-hint">
          <span className="text-slate-300">Shortcuts:</span>{' '}
          <kbd className="text-slate-300">1–6</kbd> tabs · <kbd className="text-slate-300">l</kbd> logs ·{' '}
          <kbd className="text-slate-300">e</kbd> events · <kbd className="text-slate-300">o</kbd> overview ·{' '}
          <kbd className="text-slate-300">s</kbd> shell · <kbd className="text-slate-300">?</kbd> toggle
        </div>
      ) : null}

      {/* Tabs */}
      <div className="flex flex-wrap gap-2 px-4 py-3 glass-divider-b">
        {tabs.map(tab => (
          <button
            key={tab.id}
            type="button"
            data-testid={`workload-tab-${tab.id}`}
            onClick={() => setActiveTab(tab.id)}
            className={`glass-tab tab-chip ${activeTab === tab.id ? 'glass-tab-active tab-chip-active' : ''}`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="p-4">
        {activeTab === 'overview' && (
          <div>
            {isKubeWorkload && (
              <div className="mb-4 space-y-4">
                <FixItPanel workload={workload} onAction={handleFixAction} />
                <AiTroubleshootPanel workload={workload} compact onApplied={onAction} />
              </div>
            )}
            {/* Action Buttons */}
            {isAetherManaged && canMutate ? (
              <div className="flex gap-2 mb-4 flex-wrap">
                {showShell ? (
                  <button
                    data-testid="workload-shell-button"
                    onClick={() => {
                      setShellOutput('');
                      setShellPod('');
                      setShellContainer('');
                      setShellOpen(true);
                    }}
                    className="px-3 py-1.5 text-sm font-medium rounded bg-emerald-600/20 text-emerald-400 hover:bg-emerald-600/40 border border-emerald-600/30 transition-colors"
                  >
                    Shell
                  </button>
                ) : null}
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
                        : 'glass-inset-surface text-slate-300 hover:bg-white/[0.08] border glass-divider'
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
                  <p data-testid="workload-build-result" className="w-full text-xs text-slate-400 mt-1">
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
                  {canShellDiscovered ? (
                    <button
                      data-testid="workload-shell-button"
                      onClick={() => {
                        setShellOutput('');
                        setShellPod('');
                        setShellContainer('');
                        setShellOpen(true);
                      }}
                      className="px-3 py-1.5 text-sm font-medium rounded bg-emerald-600/20 text-emerald-400 hover:bg-emerald-600/40 border border-emerald-600/30 transition-colors"
                    >
                      Shell
                    </button>
                  ) : null}
                  <button
                    onClick={() => handleAction('restart')}
                    disabled={!!actionLoading}
                    className="px-3 py-1.5 text-sm font-medium rounded glass-inset-surface text-slate-300 hover:bg-white/[0.08] border glass-divider disabled:opacity-50"
                  >
                    {actionLoading === 'restart' ? '...' : 'Restart'}
                  </button>
                  {isScalableClusterWorkload && (
                    <>
                      <div>
                        <label className="mb-1 block text-xs text-slate-500">Replicas</label>
                        <input
                          type="number"
                          min={0}
                          value={replicasInput}
                          onChange={(e) => setReplicasInput(e.target.value)}
                          className="w-24 rounded border glass-divider glass-inset-surface px-2 py-1.5 text-sm text-white"
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
              <div><span className="text-slate-500">Runtime</span><p className="text-white">{workload.runtime}</p></div>
              <div><span className="text-slate-500">Image</span><p className="text-white font-mono text-xs">{workload.image}</p></div>
              <div><span className="text-slate-500">Status</span><p className="text-white">{workload.status}</p></div>
              <div><span className="text-slate-500">Created</span><p className="text-white">{workload.created_at?.slice(0, 19)}</p></div>
              {workload.cluster && <div><span className="text-slate-500">Cluster</span><p className="text-white">{workload.cluster}</p></div>}
              {workload.namespace && <div><span className="text-slate-500">Namespace</span><p className="text-white">{workload.namespace}</p></div>}
              {workload.kind && <div><span className="text-slate-500">Kind</span><p className="text-white">{workload.kind}</p></div>}
              <div><span className="text-slate-500">Source</span><p className="text-white capitalize">{workload.source ?? 'aether'}</p></div>
              {!isAetherManaged && clusterHealth ? (
                <div className="col-span-2" data-testid="workload-health-summary">
                  <span className="text-slate-500">Cluster health</span>
                  <p className="text-white">
                    {clusterHealth.summary}{' '}
                    <span className="text-slate-400">
                      ({clusterHealth.ready_pods}/{clusterHealth.total_pods} ready
                      {clusterHealth.warning_events > 0 ? ` · ${clusterHealth.warning_events} warnings` : ''})
                    </span>
                  </p>
                </div>
              ) : null}
            </div>

            {!isAetherManaged && clusterDetail ? (
              <div className="mt-3 flex flex-wrap gap-2" data-testid="workload-manifest-export">
                <button type="button" onClick={() => void copyManifestYaml()} className="btn-secondary !px-3 !py-1.5 !text-xs">
                  {yamlCopied ? 'Copied JSON' : 'Copy manifest JSON'}
                </button>
                <button type="button" onClick={downloadManifestYaml} className="btn-secondary !px-3 !py-1.5 !text-xs">
                  Download JSON
                </button>
              </div>
            ) : null}

            {!isAetherManaged && clusterDetail && clusterDetail.pods.length > 0 ? (
              <div className="mt-4" data-testid="workload-pod-info">
                <h4 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">Pods</h4>
                <div className="overflow-x-auto rounded-lg border glass-divider">
                  <table className="w-full min-w-[28rem] text-left text-xs">
                    <thead>
                      <tr className="glass-inset-surface text-slate-500">
                        <th className="px-3 py-2 font-medium">Name</th>
                        <th className="px-3 py-2 font-medium">Phase</th>
                        <th className="px-3 py-2 font-medium">Ready</th>
                        <th className="px-3 py-2 font-medium">Restarts</th>
                        <th className="px-3 py-2 font-medium">Containers</th>
                        <th className="px-3 py-2 font-medium">Node</th>
                        {canMutate && workload.kind !== 'Pod' ? (
                          <th className="px-3 py-2 font-medium" aria-label="Pod actions" />
                        ) : null}
                      </tr>
                    </thead>
                    <tbody>
                      {clusterDetail.pods.map((pod) => (
                        <tr key={pod.name} className="glass-divider-t">
                          <td className="px-3 py-2 font-mono text-slate-200">{pod.name}</td>
                          <td className="px-3 py-2 text-slate-300">{pod.phase}</td>
                          <td className="px-3 py-2 text-slate-300">{pod.ready}/{pod.total_containers}</td>
                          <td className="px-3 py-2 text-slate-300">{pod.restarts}</td>
                          <td className="px-3 py-2 text-slate-400 truncate max-w-[10rem]" title={(pod.containers ?? []).join(', ')}>
                            {(pod.containers ?? []).join(', ') || '—'}
                          </td>
                          <td className="px-3 py-2 text-slate-400">{pod.node ?? '—'}</td>
                          {canMutate && workload.kind !== 'Pod' ? (
                            <td className="px-3 py-2 text-right">
                              <button
                                type="button"
                                onClick={() => void handleDeletePod(pod.name)}
                                disabled={!!actionLoading}
                                className="rounded px-2 py-0.5 text-[11px] text-slate-500 transition-colors hover:bg-red-500/10 hover:text-red-400 disabled:opacity-50"
                                title="Delete pod (controller recreates it)"
                                data-testid={`workload-pod-delete-${pod.name}`}
                              >
                                {actionLoading === `delete-pod-${pod.name}` ? '...' : 'Recycle'}
                              </button>
                            </td>
                          ) : null}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ) : null}

            {!isAetherManaged && (clusterMetrics.length > 0 || clusterEvents.length > 0) ? (
              <div className="mt-4" data-testid="workload-live-ops">
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-xs font-semibold uppercase tracking-wider text-slate-500">Live ops</span>
                  <button
                    type="button"
                    onClick={() => setOpsAutoRefresh((v) => !v)}
                    className={`glass-tab tab-chip !px-2.5 !py-1 !text-[11px] ${opsAutoRefresh ? 'glass-tab-active tab-chip-active' : ''}`}
                    title={opsAutoRefresh ? 'Auto-refresh on (15s)' : 'Auto-refresh paused'}
                    data-testid="workload-ops-autorefresh"
                  >
                    {opsAutoRefresh ? 'Live · 15s' : 'Paused'}
                  </button>
                </div>
                <div className="grid gap-4 lg:grid-cols-2">
                {clusterMetrics.length > 0 ? (
                  <div data-testid="workload-live-metrics">
                    <h4 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">Live usage</h4>
                    <div className="overflow-x-auto rounded-lg border glass-divider">
                      <table className="w-full min-w-[16rem] text-left text-xs">
                        <thead>
                          <tr className="glass-inset-surface text-slate-500">
                            <th className="px-3 py-2 font-medium">Pod</th>
                            <th className="px-3 py-2 font-medium">CPU</th>
                            <th className="px-3 py-2 font-medium">Memory</th>
                          </tr>
                        </thead>
                        <tbody>
                          {clusterMetrics.map((metric) => (
                            <tr key={metric.name} className="glass-divider-t">
                              <td className="px-3 py-2 font-mono text-slate-200 truncate max-w-[12rem]" title={metric.name}>{metric.name}</td>
                              <td className="px-3 py-2 text-slate-300">{metric.cpu}</td>
                              <td className="px-3 py-2 text-slate-300">{metric.memory}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </div>
                ) : null}
                {clusterEvents.length > 0 ? (
                  <div data-testid="workload-recent-events">
                    <h4 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">Recent cluster events</h4>
                    <div className="max-h-40 space-y-1.5 overflow-auto rounded-lg border glass-divider p-2">
                      {clusterEvents.map((event, index) => (
                        <div key={`${event.timestamp}-${event.reason}-${index}`} className="rounded glass-inset-surface px-2.5 py-1.5">
                          <div className="flex items-center gap-2 text-[11px]">
                            <span className={event.type_ === 'Warning' ? 'text-amber-400' : 'text-emerald-400'}>
                              {event.type_}
                            </span>
                            <span className="font-medium text-slate-200">{event.reason}</span>
                            <span className="ml-auto text-slate-600">{event.timestamp?.slice(0, 19)}</span>
                          </div>
                          <p className="mt-0.5 truncate text-[11px] text-slate-400" title={event.message}>{event.message}</p>
                        </div>
                      ))}
                    </div>
                  </div>
                ) : null}
                </div>
              </div>
            ) : null}

            {!isAetherManaged && workload.kind === 'HelmRelease' && helmHistory.length > 0 ? (
              <div className="mt-4 rounded-lg border glass-divider p-3" data-testid="workload-helm-history">
                <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                  <h4 className="text-xs font-semibold uppercase tracking-wider text-slate-500">Helm revisions</h4>
                  {canMutate ? (
                    <div className="flex items-center gap-2">
                      <select
                        value={helmRevision}
                        onChange={(e) => setHelmRevision(e.target.value)}
                        className="glass-select !py-1 !px-2 text-xs"
                        data-testid="workload-helm-revision"
                      >
                        {helmHistory.map((rev) => (
                          <option key={rev.revision} value={rev.revision}>#{rev.revision} · {rev.status}</option>
                        ))}
                      </select>
                      <button
                        type="button"
                        onClick={() => void handleHelmRollback()}
                        disabled={!!actionLoading || !helmRevision}
                        className="btn-secondary !px-3 !py-1.5 !text-xs disabled:opacity-50"
                        data-testid="workload-helm-rollback"
                      >
                        {actionLoading === 'helm-rollback' ? '...' : 'Rollback'}
                      </button>
                    </div>
                  ) : null}
                </div>
                <div className="max-h-32 overflow-auto text-xs">
                  {helmHistory.slice(0, 8).map((rev) => (
                    <div key={rev.revision} className="flex flex-wrap gap-2 glass-divider-t py-1 text-slate-400">
                      <span className="font-mono text-slate-300">#{rev.revision}</span>
                      <span>{rev.chart}</span>
                      <span className={rev.status === 'deployed' ? 'text-emerald-400' : ''}>{rev.status}</span>
                      <span className="ml-auto text-slate-600">{rev.updated?.slice(0, 19)}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}

            {!isAetherManaged && isRolloutClusterKind(workload.kind) ? (
              <div className="mt-4 rounded-lg border glass-divider p-3" data-testid="workload-rollout">
                <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                  <h4 className="text-xs font-semibold uppercase tracking-wider text-slate-500">Rollout</h4>
                  {rollout ? (
                    <span className="text-xs text-slate-400">{rollout.status || 'unknown'}</span>
                  ) : (
                    <span className="text-xs text-slate-600">Loading…</span>
                  )}
                </div>
                {canMutate ? (
                  <div className="flex flex-wrap gap-2">
                    {(['restart', 'undo', 'pause', 'resume'] as const).map((action) => (
                      <button
                        key={action}
                        type="button"
                        onClick={() => void handleRolloutAction(action)}
                        disabled={!!actionLoading}
                        className="btn-secondary !px-3 !py-1.5 !text-xs capitalize disabled:opacity-50"
                        data-testid={`workload-rollout-${action}`}
                      >
                        {actionLoading === `rollout-${action}` ? '...' : action}
                      </button>
                    ))}
                  </div>
                ) : null}
                {rollout && rollout.history.length > 0 ? (
                  <div className="mt-3 max-h-28 overflow-auto text-xs">
                    {rollout.history.slice(0, 5).map((rev) => (
                      <div key={rev.revision} className="flex gap-2 glass-divider-t py-1 text-slate-400">
                        <span className="font-mono text-slate-300">#{rev.revision}</span>
                        <span className="truncate">{rev.change_cause || '—'}</span>
                      </div>
                    ))}
                  </div>
                ) : null}
              </div>
            ) : null}

            {!isAetherManaged && canMutate && canPortForwardClusterKind(workload.kind) && (workload.kind === 'Service' || (clusterDetail?.pods.length ?? 0) > 0 || portForwardPod) ? (
              <div className="mt-4 rounded-lg border glass-divider p-3" data-testid="workload-port-forward">
                <h4 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">Port forward</h4>
                <div className="grid grid-cols-1 gap-2 sm:grid-cols-4">
                  <select
                    value={workload.kind === 'Service' ? clusterResourceName : portForwardPod}
                    onChange={(e) => setPortForwardPod(e.target.value)}
                    disabled={workload.kind === 'Service' || !!portForwardSession}
                    className="glass-select text-xs disabled:opacity-60"
                    data-testid="workload-pf-pod"
                  >
                    {(workload.kind === 'Service'
                      ? [clusterResourceName]
                      : (clusterDetail?.pods.map((p) => p.name) ?? (portForwardPod ? [portForwardPod] : []))
                    ).map((name) => (
                      <option key={name} value={name}>{name}</option>
                    ))}
                  </select>
                  <input
                    type="number"
                    value={portForwardRemotePort}
                    onChange={(e) => setPortForwardRemotePort(e.target.value)}
                    className="glass-input text-xs"
                    placeholder="Remote port"
                    data-testid="workload-pf-remote"
                    disabled={!!portForwardSession}
                  />
                  <input
                    type="number"
                    value={portForwardLocalPort}
                    onChange={(e) => setPortForwardLocalPort(e.target.value)}
                    className="glass-input text-xs"
                    placeholder="Local (optional)"
                    data-testid="workload-pf-local"
                    disabled={!!portForwardSession}
                  />
                  <div className="flex gap-2">
                    <button
                      type="button"
                      onClick={() => void handlePortForwardStart()}
                      disabled={!!portForwardSession || !!actionLoading || (!portForwardPod && workload.kind !== 'Service')}
                      className="flex-1 rounded-lg border border-blue-700/40 bg-blue-900/20 px-2 py-1.5 text-xs text-blue-200 hover:bg-blue-800/30 disabled:opacity-50"
                      data-testid="workload-pf-start"
                    >
                      {actionLoading === 'port-forward' ? '...' : 'Start'}
                    </button>
                    <button
                      type="button"
                      onClick={() => void handlePortForwardStop()}
                      disabled={!portForwardSession || !!actionLoading}
                      className="flex-1 btn-secondary !px-2 !py-1.5 !text-xs disabled:opacity-50"
                      data-testid="workload-pf-stop"
                    >
                      {actionLoading === 'port-forward-stop' ? '...' : 'Stop'}
                    </button>
                  </div>
                </div>
                {portForwardSession ? (
                  <p className="mt-2 text-xs text-emerald-300" data-testid="workload-pf-active">
                    Active: {portForwardSession.local_url} → {portForwardSession.target_kind}/{portForwardSession.target_name}:{portForwardSession.remote_port}
                  </p>
                ) : null}
              </div>
            ) : null}

            {!isAetherManaged && workload.cluster ? (() => {
              const cmds = buildKubectlCommands({
                kind: workload.kind,
                namespace: workload.namespace,
                resourceName: clusterResourceName,
                podName:
                  shellPod ||
                  pickExecPodName(
                    workload.kind ?? undefined,
                    clusterResourceName,
                    clusterDetail?.pods.map((p) => ({ name: p.name, phase: p.phase })) ?? [],
                  ),
                context: workload.cluster,
              });
              if (cmds.length === 0) return null;
              return (
                <div className="mt-4" data-testid="workload-kubectl">
                  <h4 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">
                    Run in your terminal
                  </h4>
                  <div className="space-y-1.5">
                    {cmds.map((cmd) => (
                      <div
                        key={cmd.label}
                        className="flex items-center gap-2 rounded-lg glass-inset-surface px-3 py-2"
                      >
                        <span className="w-24 shrink-0 text-[11px] font-medium uppercase tracking-wider text-slate-500">
                          {cmd.label}
                        </span>
                        <code className="min-w-0 flex-1 truncate font-mono text-xs text-slate-300" title={cmd.command}>
                          {cmd.command}
                        </code>
                        <button
                          type="button"
                          onClick={() => {
                            void navigator.clipboard.writeText(cmd.command);
                            setCopiedCmd(cmd.label);
                            window.setTimeout(() => setCopiedCmd(''), 1500);
                          }}
                          className="shrink-0 rounded px-2 py-1 text-[11px] text-slate-400 transition-colors hover:bg-white/5 hover:text-aether"
                        >
                          {copiedCmd === cmd.label ? 'Copied' : 'Copy'}
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              );
            })() : null}

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
                  { label: 'Envs', slug: 'envs', path: pathWithQuery(viewToPath('envs'), { q: workload.name }) },
                  { label: 'Intelligence', slug: 'intelligence', path: pathWithQuery(viewToPath('intelligence'), { tab: 'predictions', workload: workload.name }) },
                ] as const
              ).map((link) => (
                <a
                  key={link.label}
                  href={link.path}
                  data-testid={`workload-link-${link.slug}`}
                  className="rounded-lg border glass-divider px-2.5 py-1 text-xs text-slate-300 hover:border-aether/40 hover:text-aether transition-colors"
                >
                  {link.label}
                </a>
              ))}
            </div>

            {isAetherManaged ? (
              <div className="mt-4 glass-drawer p-3" data-testid="workload-snapshots">
                <h4 className="mb-2 text-sm font-semibold text-slate-200">Snapshots</h4>
                {snapshotsLoading ? (
                  <p className="text-xs text-slate-500">Loading snapshots…</p>
                ) : snapshots.length === 0 ? (
                  <p className="text-xs text-slate-500">No snapshots yet — snapshots are created before migrations and updates.</p>
                ) : (
                  <ul className="space-y-2">
                    {snapshots.map((snap) => (
                      <li key={snap.version} className="flex flex-wrap items-center justify-between gap-2 text-sm">
                        <span className="font-mono text-xs text-slate-400 truncate" title={snap.path}>
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
              <div className="mt-4 glass-drawer p-3">
                <h4 className="mb-3 text-sm font-semibold text-slate-200">Network policies</h4>
                <dl className="space-y-2 text-sm">
                  <div className="flex justify-between gap-4">
                    <dt className="text-slate-500">NetworkPolicy</dt>
                    <dd className="font-mono text-xs text-slate-200">{networkPolicyName}</dd>
                  </div>
                  <div className="flex justify-between gap-4">
                    <dt className="text-slate-500">Cilium CNP</dt>
                    <dd className="font-mono text-xs text-slate-200">{ciliumPolicyName}</dd>
                  </div>
                </dl>
                <a
                  href={clusterBrowseNetworkPath}
                  className="mt-3 inline-flex items-center gap-1.5 text-xs text-aether hover:underline"
                >
                  Browse cluster network policies
                  <Link2 className="h-3.5 w-3.5" />
                </a>
                <p className="mt-2 text-xs text-slate-500">
                  Names follow Aether deploy conventions when <code>network.networkPolicy</code> or{' '}
                  <code>network.ciliumNetworkPolicy</code> is set in the workload spec.
                </p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'logs' && (
          <LogViewer workloadName={workload.name} logsPath={clusterLogsPath} containers={logContainers} />
        )}

        {activeTab === 'topology' && isKubeWorkload && (
          <ApplicationTopology workload={workload} />
        )}

        {activeTab === 'manifest' && (
          <div className="space-y-4">
            {isAetherManaged ? (
              <p className="text-slate-500">Manifest inspection is currently available for Kubernetes resources discovered directly from the cluster.</p>
            ) : !clusterDetail ? (
              <p className="text-slate-500">Loading Kubernetes resource details...</p>
            ) : (
              <>
                <div className="grid grid-cols-2 gap-3 text-sm">
                  <div><span className="text-slate-500">API Version</span><p className="text-white">{clusterDetail.api_version ?? 'unknown'}</p></div>
                  <div><span className="text-slate-500">Pods</span><p className="text-white">{clusterDetail.pods.length}</p></div>
                </div>

                {clusterDetail.conditions.length > 0 && (
                  <div className="glass-drawer p-3">
                    <h4 className="mb-3 text-sm font-semibold text-slate-200">Conditions</h4>
                    <div className="space-y-2">
                      {clusterDetail.conditions.map((condition) => (
                        <div key={`${condition.type_}:${condition.reason ?? 'none'}`} className="glass-table-row rounded-md px-3 py-2 text-sm">
                          <div className="flex items-center justify-between">
                            <span className="font-medium text-slate-100">{condition.type_}</span>
                            <span className={condition.status === 'True' ? 'text-emerald-400' : 'text-amber-400'}>
                              {condition.status}
                            </span>
                          </div>
                          {condition.reason && <div className="mt-1 text-xs text-slate-400">{condition.reason}</div>}
                          {condition.message && <div className="mt-1 text-xs text-slate-500">{condition.message}</div>}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {clusterDetail.pods.length > 0 && (
                  <div className="glass-drawer p-3">
                    <h4 className="mb-3 text-sm font-semibold text-slate-200">Pods</h4>
                    <div className="space-y-2">
                      {clusterDetail.pods.map((pod) => (
                        <div key={pod.name} className="grid grid-cols-5 gap-3 glass-table-row rounded-md px-3 py-2 text-sm">
                          <div className="col-span-2">
                            <div className="text-slate-100">{pod.name}</div>
                            <div className="text-xs text-slate-500">{pod.node ?? 'node unknown'}</div>
                          </div>
                          <div className="text-slate-300">{pod.phase}</div>
                          <div className="text-slate-300">{pod.ready}/{pod.total_containers} ready</div>
                          <div className="text-slate-300">{pod.restarts} restarts</div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                <div className="glass-drawer p-3">
                  <h4 className="mb-3 text-sm font-semibold text-slate-200">Manifest</h4>
                  <pre className="glass-code-block-body max-h-96 overflow-auto text-xs">
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
              <p className="text-slate-500">Drift analysis is currently available only for Aether-managed workloads.</p>
            ) : driftData ? (
              <div>
                <div className={`text-sm font-medium mb-2 ${(driftData as Record<string, unknown>).has_drift ? 'text-orange-400' : 'text-emerald-400'}`}>
                  {(driftData as Record<string, unknown>).has_drift
                    ? `${((driftData as Record<string, unknown>).drifts as unknown[])?.length || 0} drift item(s) detected`
                    : 'No drift detected'}
                </div>
                {((driftData as Record<string, unknown>).drifts as Array<Record<string, string>>)?.map((d, i: number) => (
                  <div key={i} className="glass-inset-surface rounded p-2 mb-2 text-sm">
                    <span className={`font-medium ${d.severity === 'Critical' ? 'text-red-400' : d.severity === 'Warning' ? 'text-orange-400' : 'text-blue-400'}`}>
                      [{d.severity}]
                    </span>
                    {' '}<span className="text-slate-300">{d.field}</span>
                    <div className="text-slate-500 mt-1">Expected: {d.expected} | Actual: {d.actual}</div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-slate-500">Loading drift data...</p>
            )}
          </div>
        )}

        {activeTab === 'scoring' && (
          <div className="text-sm text-slate-400">
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
            <div className="mt-6 glass-divider-t pt-4">
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
              <p className="text-slate-500">Loading events...</p>
            ) : events.length === 0 ? (
              <p className="text-slate-500">No events recorded for this workload.</p>
            ) : (
              <div className="space-y-2 max-h-96 overflow-auto">
                {events.map((ev, i) => (
                  <div key={`${ev.timestamp}-${i}`} className="flex items-start gap-3 glass-drawer p-3">
                    <SeverityBadge severity={ev.severity} />
                    <div className="flex-1 min-w-0">
                      <div className="font-medium text-slate-200">{ev.title}</div>
                      <div className="text-xs text-slate-500 mt-0.5">{ev.message}</div>
                      <div className="text-xs text-slate-600 mt-1">{ev.timestamp?.slice(0, 19)}</div>
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
        <div className={`fixed inset-0 flex items-center justify-center z-[60] ${shellFullscreen ? 'p-0' : ''}`}>
          <div className="glass-modal-backdrop absolute inset-0" aria-hidden />
          <div className={`glass-modal-panel relative overflow-hidden transition-all ${shellFullscreen ? 'h-full w-full max-w-none rounded-none' : 'mx-4 w-full max-w-4xl'}`}>
            <div className="flex items-center justify-between glass-divider-b glass-inset-surface px-5 py-3">
              <div className="flex flex-wrap items-center gap-3">
                <div className="font-medium" data-testid="workload-shell-title">
                  Shell — {shellPod || workload.name}
                </div>
                <div className={`text-xs px-2 py-0.5 rounded ${shellConnected ? 'bg-emerald-500/20 text-emerald-400' : 'glass-inset-surface text-slate-400'}`}>
                  {shellConnected ? 'Connected' : 'Disconnected'}
                </div>
                {shellPods.length > 1 ? (
                  <label className="flex items-center gap-1 text-xs text-slate-400">
                    Pod
                    <select
                      value={shellPod}
                      onChange={(e) => {
                        setShellOutput('');
                        setShellContainer('');
                        setShellPod(e.target.value);
                      }}
                      className="glass-input !py-1 !px-2 text-xs max-w-[14rem]"
                      data-testid="workload-shell-pod-select"
                    >
                      {shellPods.map((pod) => (
                        <option key={pod.name} value={pod.name}>
                          {pod.name} ({pod.phase})
                        </option>
                      ))}
                    </select>
                  </label>
                ) : null}
                {(shellPods.find((pod) => pod.name === shellPod)?.containers.length ?? 0) > 1 ? (
                  <label className="flex items-center gap-1 text-xs text-slate-400">
                    Container
                    <select
                      value={shellContainer}
                      onChange={(e) => {
                        setShellOutput('');
                        setShellContainer(e.target.value);
                      }}
                      className="glass-input !py-1 !px-2 text-xs max-w-[10rem]"
                      data-testid="workload-shell-container-select"
                    >
                      {(shellPods.find((pod) => pod.name === shellPod)?.containers ?? []).map((name) => (
                        <option key={name} value={name}>{name}</option>
                      ))}
                    </select>
                  </label>
                ) : null}
              </div>

              <div className="flex items-center gap-2">
                <button 
                  onClick={() => navigator.clipboard.writeText(shellOutput)}
                  className="text-xs px-3 py-1 rounded glass-inset-surface glass-inset-hover text-slate-300"
                >
                  Copy
                </button>
                <button 
                  onClick={() => setShellOutput('')}
                  className="text-xs px-3 py-1 rounded glass-inset-surface glass-inset-hover text-slate-300"
                >
                  Clear
                </button>
                <button 
                  onClick={() => setShellFullscreen(!shellFullscreen)}
                  className="text-xs px-3 py-1 rounded glass-inset-surface glass-inset-hover text-slate-300"
                >
                  {shellFullscreen ? 'Exit Fullscreen' : 'Fullscreen'}
                </button>
                <button 
                  onClick={() => {
                    shellSocketRef.current?.close();
                    setShellOpen(false);
                    setShellConnected(false);
                    setShellOutput('');
                    setShellPod('');
                    setShellPods([]);
                    setShellContainer('');
                    setShellFullscreen(false);
                  }} 
                  className="text-slate-400 hover:text-white text-xl leading-none ml-1"
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
                data-testid="workload-shell-output"
                className="glass-code-block-body h-[420px] overflow-auto whitespace-pre-wrap font-mono text-sm text-emerald-400 shadow-inner"
              >
                {shellOutput || (shellPod ? '[aether] Connecting to pod shell...\n' : '[aether] Resolving pod...\n')}
              </div>

              <div className="mt-4 flex gap-2 items-center">
                <select 
                  className="glass-select text-sm text-slate-400"
                  value={shellCommand}
                  onChange={(e) => {
                    setShellOutput('');
                    setShellCommand(e.target.value);
                  }}
                  data-testid="workload-shell-command-select"
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
                  className="glass-input flex-1 font-mono text-sm focus:border-emerald-600"
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

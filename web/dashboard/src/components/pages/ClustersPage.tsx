// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useMemo, useRef, useState, useCallback } from 'react';
import { Link, useSearchParams } from 'react-router';
import { Container, Plus, RefreshCw, Save, Trash2 } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';
import { apiFetch, apiFetchSettled, apiPost, apiWebSocketUrl } from '../../utils/api';
import Modal from '../Modal';
import LogViewer from '../LogViewer';
import EmptyState from '../EmptyState';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import PageTabs from '../PageTabs';
import StatCard from '../StatCard';
import CodeBlock from '../CodeBlock';
import EventCorrelationPanel from '../EventCorrelationPanel';
import { setClusterContext } from '../../utils/clusterContext';
import { formatTimestamp } from '../../utils/formatters';
import type {
  AuthStatus,
  ClusterBrowseItem,
  ClusterDiffLine,
  ClusterHealthSummary,
  ClusterMetricsSummary,
  ClusterTopMetric,
  ClusterPortForwardSession,
  ClusterRelatedEvent,
  HelmRevisionEntry,
  ClusterNamespaceSummary,
  ClusterResourceDetail,
  ClusterRolloutStatus,
  ClusterSummary,
  AuditEvent,
  CiliumStatusResponse,
} from '../../types/api';

const kindOptions = ['Namespace', 'Node', 'PersistentVolume', 'StorageClass', 'Pod', 'ServiceAccount', 'Secret', 'PersistentVolumeClaim', 'ResourceQuota', 'LimitRange', 'Deployment', 'StatefulSet', 'DaemonSet', 'Job', 'CronJob', 'HorizontalPodAutoscaler', 'Service', 'EndpointSlice', 'Ingress', 'NetworkPolicy', 'CiliumNetworkPolicy', 'CiliumClusterwideNetworkPolicy', 'ConfigMap', 'Event', 'HelmRelease', 'DataVolume', 'VirtualMachine', 'VirtualMachineInstance', 'CustomResource'];

function manifestCreationTimestamp(manifest: Record<string, unknown>): string {
  const metadata = manifest.metadata;
  if (metadata && typeof metadata === 'object' && 'creationTimestamp' in metadata) {
    const value = (metadata as { creationTimestamp?: unknown }).creationTimestamp;
    return typeof value === 'string' ? value : '';
  }
  return '';
}

function defaultApiVersion(kind: string): string {
  if (['Deployment', 'StatefulSet', 'DaemonSet'].includes(kind)) return 'apps/v1';
  if (['Job', 'CronJob'].includes(kind)) return 'batch/v1';
  if (kind === 'HorizontalPodAutoscaler') return 'autoscaling/v2';
  if (kind === 'Ingress') return 'networking.k8s.io/v1';
  if (kind === 'NetworkPolicy') return 'networking.k8s.io/v1';
  if (kind === 'CiliumNetworkPolicy' || kind === 'CiliumClusterwideNetworkPolicy') return 'cilium.io/v2';
  if (kind === 'HelmRelease') return 'helm.sh/v1';
  if (['VirtualMachine', 'VirtualMachineInstance'].includes(kind)) return 'kubevirt.io/v1';
  if (kind === 'DataVolume') return 'cdi.kubevirt.io/v1beta1';
  return 'v1';
}

function manifestContainerNames(manifest: Record<string, unknown>): string[] {
  const typed = manifest as {
    spec?: {
      containers?: Array<{ name?: string }>;
      template?: { spec?: { containers?: Array<{ name?: string }> } };
    };
  };
  return (
    typed.spec?.containers?.map((container) => container.name).filter(Boolean) ??
    typed.spec?.template?.spec?.containers?.map((container) => container.name).filter(Boolean) ??
    []
  ) as string[];
}

export default function ClustersPage() {
  const panelClass = 'rounded-xl border border-zinc-800 bg-zinc-950/60 text-zinc-100';
  const [authStatus, setAuthStatus] = useState<AuthStatus | null>(null);
  const [summary, setSummary] = useState<ClusterSummary | null>(null);
  const [metricsSummary, setMetricsSummary] = useState<ClusterMetricsSummary | null>(null);
  const [cluster, setCluster] = useState('');
  const [namespace, setNamespace] = useState('all');
  const [kind, setKind] = useState('Pod');
  const [customApiVersion, setCustomApiVersion] = useState('');
  const [customPlural, setCustomPlural] = useState('');
  const [customNamespaced, setCustomNamespaced] = useState(true);
  const [namespaces, setNamespaces] = useState<ClusterNamespaceSummary[]>([]);
  const [resources, setResources] = useState<ClusterBrowseItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [bootstrapLoading, setBootstrapLoading] = useState(true);
  const [bootstrapFailed, setBootstrapFailed] = useState(false);
  const [selected, setSelected] = useState<ClusterResourceDetail | null>(null);
  const [selectedEvents, setSelectedEvents] = useState<ClusterRelatedEvent[]>([]);
  const [relatedAudit, setRelatedAudit] = useState<AuditEvent[]>([]);
  const [healthSummary, setHealthSummary] = useState<ClusterHealthSummary | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [manifestDraft, setManifestDraft] = useState('');
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [createManifestDraft, setCreateManifestDraft] = useState('');
  const [createHelmRelease, setCreateHelmRelease] = useState('');
  const [createHelmChart, setCreateHelmChart] = useState('');
  const [createHelmValues, setCreateHelmValues] = useState('');
  const [actionLoading, setActionLoading] = useState('');
  const [replicasInput, setReplicasInput] = useState('1');
  const [watchConnected, setWatchConnected] = useState(false);
  const [execPod, setExecPod] = useState('');
  const [execContainer, setExecContainer] = useState('');
  const [execCommand, setExecCommand] = useState('/bin/sh');
  const [execInput, setExecInput] = useState('');
  const [execOutput, setExecOutput] = useState('');
  const [execConnected, setExecConnected] = useState(false);
  const [portForwardPod, setPortForwardPod] = useState('');
  const [portForwardRemotePort, setPortForwardRemotePort] = useState('8080');
  const [portForwardLocalPort, setPortForwardLocalPort] = useState('');
  const [portForwardSession, setPortForwardSession] = useState<ClusterPortForwardSession | null>(null);
  const [rollout, setRollout] = useState<ClusterRolloutStatus | null>(null);
  const [rolloutRevision, setRolloutRevision] = useState('');
  const [topMetrics, setTopMetrics] = useState<ClusterTopMetric[]>([]);
  const [helmHistory, setHelmHistory] = useState<HelmRevisionEntry[]>([]);
  const [helmChart, setHelmChart] = useState('');
  const [helmValues, setHelmValues] = useState('');
  const [helmRevision, setHelmRevision] = useState('');
  const [serverDiff, setServerDiff] = useState<ClusterDiffLine[]>([]);
  const [terminalExpanded, setTerminalExpanded] = useState(false);
  const [detailTab, setDetailTab] = useState<'overview' | 'events' | 'logs' | 'terminal' | 'manifest'>('overview');
  const [pageTab, setPageTab] = useState<'browse' | 'network'>('browse');
  const [ciliumStatus, setCiliumStatus] = useState<CiliumStatusResponse | null>(null);
  const [searchParams, setSearchParams] = useSearchParams();
  function setPageTabWithUrl(tab: 'browse' | 'network') {
    setPageTab(tab);
    setSearchParams(
      (prev) => {
        const copy = new URLSearchParams(prev);
        if (tab === 'browse') copy.delete('tab');
        else copy.set('tab', tab);
        return copy;
      },
      { replace: true },
    );
  }

  const watchSocketRef = useRef<WebSocket | null>(null);
  const execSocketRef = useRef<WebSocket | null>(null);

  const selectedContainers = useMemo(() => {
    if (!selected) return [];
    return manifestContainerNames(selected.manifest);
  }, [selected]);

  const canMutateCluster = authStatus?.role === 'admin' || authStatus?.role === 'operator' || authStatus === null;
  const canDeleteCluster = authStatus?.role === 'admin' || authStatus === null;

  function toast(message: string, type: 'success' | 'error') {
    window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
  }

  async function loadNetworkPolicies(targetCluster = cluster, targetNamespace = namespace) {
    if (!targetCluster) return;
    setLoading(true);
    const ns = encodeURIComponent(targetNamespace);
    const cl = encodeURIComponent(targetCluster);
    const [np, cnp, ccnp] = await Promise.all([
      apiFetch<ClusterBrowseItem[]>(`/cluster/browse?cluster=${cl}&namespace=${ns}&kind=NetworkPolicy`),
      apiFetch<ClusterBrowseItem[]>(`/cluster/browse?cluster=${cl}&namespace=${ns}&kind=CiliumNetworkPolicy`),
      apiFetch<ClusterBrowseItem[]>(`/cluster/browse?cluster=${cl}&namespace=_cluster&kind=CiliumClusterwideNetworkPolicy`),
    ]);
    setResources([...(np ?? []), ...(cnp ?? []), ...(ccnp ?? [])]);
    setLoading(false);
  }

  async function loadResources(targetCluster = cluster, targetNamespace = namespace, targetKind = kind) {
    if (!targetCluster) return;
    setLoading(true);
    const customParams = targetKind === 'CustomResource'
      ? `&api_version=${encodeURIComponent(customApiVersion)}&plural=${encodeURIComponent(customPlural)}&namespaced=${customNamespaced ? 'true' : 'false'}`
      : '';
    const data = await apiFetch<ClusterBrowseItem[]>(
      `/cluster/browse?cluster=${encodeURIComponent(targetCluster)}&namespace=${encodeURIComponent(targetNamespace)}&kind=${encodeURIComponent(targetKind)}${customParams}`
    );
    setResources(data ?? []);
    setLoading(false);
  }

  const loadBootstrap = useCallback(async () => {
    setBootstrapLoading(true);
    setBootstrapFailed(false);
    const [authRes, summaryRes] = await Promise.all([
      apiFetchSettled<AuthStatus>('/auth/me'),
      apiFetchSettled<ClusterSummary>('/cluster/summary'),
    ]);
    if (authRes.ok) setAuthStatus(authRes.data);
    if (summaryRes.ok) {
      const data = summaryRes.data;
      setSummary(data);
      const urlCluster = searchParams.get('cluster');
      const urlNamespace = searchParams.get('namespace');
      const urlKind = searchParams.get('kind');
      const urlTab = searchParams.get('tab');
      const firstCluster = urlCluster && data?.clusters.some((c) => c.name === urlCluster)
        ? urlCluster
        : data?.clusters.find((item) => item.reachable)?.name ?? data?.clusters[0]?.name ?? '';
      setCluster(firstCluster);
      if (urlNamespace) setNamespace(urlNamespace);
      if (urlKind && kindOptions.includes(urlKind)) setKind(urlKind);
      if (urlTab === 'network') setPageTab('network');
    } else {
      setBootstrapFailed(true);
      setSummary(null);
    }
    setBootstrapLoading(false);
    setLoading(false);
  }, [searchParams]);

  useEffect(() => {
    void loadBootstrap();
  }, [loadBootstrap]);

  useEffect(() => {
    if (!cluster) return;
    apiFetch<ClusterNamespaceSummary[]>(`/cluster/namespaces?cluster=${encodeURIComponent(cluster)}`).then((data) => {
      setNamespaces(data ?? []);
    });
  }, [cluster]);

  useEffect(() => {
    if (!cluster) return;
    apiFetch<ClusterMetricsSummary>(`/cluster/metrics/summary?cluster=${encodeURIComponent(cluster)}&namespace=${encodeURIComponent(namespace)}`).then((data) => {
      setMetricsSummary(data);
    });
  }, [cluster, namespace]);

  useEffect(() => {
    if (!cluster) return;
    apiFetch<CiliumStatusResponse>(
      `/cluster/cilium/status?cluster=${encodeURIComponent(cluster)}&namespace=${encodeURIComponent(namespace === 'all' ? 'aether-system' : namespace)}`
    ).then((data) => setCiliumStatus(data ?? null));
  }, [cluster, namespace]);

  useEffect(() => {
    if (!selected) {
      setServerDiff([]);
      return;
    }

    try {
      const draftManifest = JSON.parse(manifestDraft);
      apiPost<ClusterDiffLine[]>('/cluster/diff', {
        cluster: selected.cluster,
        namespace: selected.namespace,
        kind: selected.kind,
        name: selected.name,
        draft_manifest: draftManifest,
        api_version: selected.kind === 'CustomResource' ? customApiVersion : undefined,
        plural: selected.kind === 'CustomResource' ? customPlural : undefined,
        namespaced: selected.kind === 'CustomResource' ? customNamespaced : undefined,
      }).then((response) => {
        if (response.success) {
          setServerDiff(response.data ?? []);
        }
      });
    } catch {
      setServerDiff([{ kind: 'remove', text: '- invalid JSON draft' }]);
    }
  }, [selected, manifestDraft]);

  useEffect(() => {
    if (!cluster) return;
    setLoading(true);
    if (pageTab === 'network') {
      void loadNetworkPolicies();
      watchSocketRef.current?.close();
      setWatchConnected(false);
      return;
    }
    loadResources();

    watchSocketRef.current?.close();
    if (kind === 'HelmRelease' || (kind === 'CustomResource' && (!customApiVersion || !customPlural))) {
      setWatchConnected(false);
      return;
    }

    const customWatchParams = kind === 'CustomResource'
      ? `&api_version=${encodeURIComponent(customApiVersion)}&plural=${encodeURIComponent(customPlural)}&namespaced=${customNamespaced ? 'true' : 'false'}`
      : '';
    const socket = new WebSocket(
      apiWebSocketUrl(
        `/cluster/ws/watch?cluster=${encodeURIComponent(cluster)}&namespace=${encodeURIComponent(namespace)}&kind=${encodeURIComponent(kind)}${customWatchParams}`
      )
    );
    watchSocketRef.current = socket;

    socket.onopen = () => setWatchConnected(true);
    socket.onclose = () => setWatchConnected(false);
    socket.onerror = () => setWatchConnected(false);
    socket.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data) as ClusterBrowseItem[] | { type: string; message: string };
        if (Array.isArray(payload)) {
          setResources(payload);
          setLoading(false);
          if (selected && selected.cluster === cluster && selected.kind === kind) {
            const manifestMatchesLive = manifestDraft === JSON.stringify(selected.manifest, null, 2);
            const next = payload.find((item) => item.name === selected.name && item.namespace === selected.namespace);
            if (manifestMatchesLive && next) {
              void openDetail(next);
            }
            if (!next) {
              setSelected(null);
              setSelectedEvents([]);
            }
          }
        }
      } catch {
        // ignore malformed watch payloads
      }
    };

    return () => {
      socket.close();
      if (watchSocketRef.current === socket) {
        watchSocketRef.current = null;
      }
      setWatchConnected(false);
    };
  }, [cluster, namespace, kind, customApiVersion, customPlural, customNamespaced, pageTab]);

  const selectedLogsPath = useMemo(() => {
    if (!selected) return undefined;
    if (!['Pod', 'Deployment', 'StatefulSet', 'DaemonSet', 'Service', 'VirtualMachine', 'VirtualMachineInstance', 'HelmRelease'].includes(selected.kind)) return undefined;
    return `/cluster/logs?cluster=${encodeURIComponent(selected.cluster)}&namespace=${encodeURIComponent(selected.namespace)}&kind=${encodeURIComponent(selected.kind)}&name=${encodeURIComponent(selected.name)}`;
  }, [selected]);

  useEffect(() => {
    const nextNamespace = kind === 'Namespace' ? '_cluster' : namespace === '_cluster' ? 'all' : namespace;
    if (nextNamespace !== namespace) {
      setNamespace(nextNamespace);
    }
  }, [kind, namespace]);

  async function openDetail(resource: ClusterBrowseItem) {
    setDetailLoading(true);
    setSelected(null);
    const canRollout = ['Deployment', 'StatefulSet', 'DaemonSet'].includes(resource.kind);
    const shouldLoadTop = ['Pod', 'Deployment', 'StatefulSet', 'DaemonSet', 'Service', 'HelmRelease'].includes(resource.kind);
    const shouldLoadHelm = resource.kind === 'HelmRelease';
    const auditFilter = encodeURIComponent(resource.name);
    const [detail, events, rolloutStatus, top, helmRevisions, health, auditResp] = await Promise.all([
      apiFetch<ClusterResourceDetail>(
        `/cluster/resource?cluster=${encodeURIComponent(resource.cluster)}&namespace=${encodeURIComponent(resource.namespace)}&kind=${encodeURIComponent(resource.kind)}&name=${encodeURIComponent(resource.name)}${resource.kind === 'CustomResource' ? `&api_version=${encodeURIComponent(customApiVersion)}&plural=${encodeURIComponent(customPlural)}&namespaced=${customNamespaced ? 'true' : 'false'}` : ''}`
      ),
      apiFetch<ClusterRelatedEvent[]>(
        `/cluster/events?cluster=${encodeURIComponent(resource.cluster)}&namespace=${encodeURIComponent(resource.namespace)}&kind=${encodeURIComponent(resource.kind)}&name=${encodeURIComponent(resource.name)}`
      ),
      canRollout
        ? apiFetch<ClusterRolloutStatus>(
            `/cluster/rollout?cluster=${encodeURIComponent(resource.cluster)}&namespace=${encodeURIComponent(resource.namespace)}&kind=${encodeURIComponent(resource.kind)}&name=${encodeURIComponent(resource.name)}`
          )
        : Promise.resolve(null),
      shouldLoadTop
        ? apiFetch<ClusterTopMetric[]>(
            `/cluster/top?cluster=${encodeURIComponent(resource.cluster)}&namespace=${encodeURIComponent(resource.namespace)}&kind=${encodeURIComponent(resource.kind)}&name=${encodeURIComponent(resource.name)}`
          )
        : Promise.resolve(null),
      shouldLoadHelm
        ? apiFetch<HelmRevisionEntry[]>(
            `/cluster/helm/history?cluster=${encodeURIComponent(resource.cluster)}&namespace=${encodeURIComponent(resource.namespace)}&release=${encodeURIComponent(resource.name)}`
          )
        : Promise.resolve(null),
      apiFetch<ClusterHealthSummary>(
        `/cluster/health?cluster=${encodeURIComponent(resource.cluster)}&namespace=${encodeURIComponent(resource.namespace)}&kind=${encodeURIComponent(resource.kind)}&name=${encodeURIComponent(resource.name)}${resource.kind === 'CustomResource' ? `&api_version=${encodeURIComponent(customApiVersion)}&plural=${encodeURIComponent(customPlural)}&namespaced=${customNamespaced ? 'true' : 'false'}` : ''}`
      ),
      apiFetch<{ recent_events: AuditEvent[] }>(`/audit?workload=${auditFilter}&limit=30`),
    ]);
    setSelected(detail);
    setSelectedEvents(events ?? []);
    setRelatedAudit(auditResp?.recent_events ?? []);
    setRollout(rolloutStatus);
    setRolloutRevision(rolloutStatus?.history[0]?.revision ?? '');
    setTopMetrics(top ?? []);
    setHelmHistory(helmRevisions ?? []);
    setHelmRevision(helmRevisions?.[0]?.revision ?? '');
    setHealthSummary(health);
    if (detail) {
      setManifestDraft(JSON.stringify(detail.manifest, null, 2));
      const replicas = (detail.manifest?.spec as { replicas?: number } | undefined)?.replicas;
      if (typeof replicas === 'number') {
        setReplicasInput(String(replicas));
      }
      const defaultPod = detail.kind === 'Pod' ? detail.name : detail.pods[0]?.name ?? '';
      setExecPod(defaultPod);
      setPortForwardPod(defaultPod);
      setExecContainer(manifestContainerNames(detail.manifest)[0] ?? '');
      const containerPort = (detail.manifest?.spec as { ports?: Array<{ port?: number }>; template?: { spec?: { containers?: Array<{ ports?: Array<{ containerPort?: number }> }> } } } | undefined);
      const defaultRemotePort = detail.kind === 'Service'
        ? containerPort?.ports?.[0]?.port
        : containerPort?.template?.spec?.containers?.[0]?.ports?.[0]?.containerPort;
      if (typeof defaultRemotePort === 'number') {
        setPortForwardRemotePort(String(defaultRemotePort));
      }
      if (detail.kind === 'HelmRelease') {
        const manifest = detail.manifest as { status?: { chart?: { metadata?: { name?: string; version?: string } } }; valuesYaml?: string };
        const chartName = manifest.status?.chart?.metadata?.name;
        const chartVersion = manifest.status?.chart?.metadata?.version;
        setHelmChart(chartName ? `${chartName}${chartVersion ? ` --version ${chartVersion}` : ''}` : '');
        setHelmValues(typeof manifest.valuesYaml === 'string' ? manifest.valuesYaml : '');
      }
    }
    setDetailLoading(false);
  }

  useEffect(() => {
    return () => {
      watchSocketRef.current?.close();
      execSocketRef.current?.close();
    };
  }, []);

  async function handleApply() {
    if (!selected) return;
    if (!canMutateCluster) {
      toast('Your role cannot apply cluster changes', 'error');
      return;
    }
    setActionLoading('apply');
    try {
      const manifest = JSON.parse(manifestDraft);
      const response = await apiPost<string>('/cluster/apply', {
        cluster: selected.cluster,
        namespace: selected.namespace,
        kind: selected.kind,
        manifest,
        api_version: selected.kind === 'CustomResource' ? customApiVersion : undefined,
        plural: selected.kind === 'CustomResource' ? customPlural : undefined,
        namespaced: selected.kind === 'CustomResource' ? customNamespaced : undefined,
      });
      if (!response.success) {
        throw new Error(response.error ?? 'apply failed');
      }
      toast(`Applied ${selected.kind} ${selected.name}`, 'success');
        await openDetail({
          cluster: selected.cluster,
          namespace: selected.namespace,
          kind: selected.kind,
          name: selected.name,
          status: '',
          created_at: manifestCreationTimestamp(selected.manifest),
          detail: null,
        });
      await loadResources();
    } catch (error) {
      toast(`Apply failed: ${String(error)}`, 'error');
    } finally {
      setActionLoading('');
    }
  }

  async function handleResourceAction(action: 'start' | 'stop' | 'restart' | 'delete' | 'scale' | 'suspend' | 'resume' | 'cordon' | 'uncordon' | 'drain') {
    if (!selected) return;
    if (action === 'delete' && !canDeleteCluster) {
      toast('Your role cannot delete cluster resources', 'error');
      return;
    }
    if (action !== 'delete' && !canMutateCluster) {
      toast(`Your role cannot ${action} cluster resources`, 'error');
      return;
    }
    setActionLoading(action);
    const response = await apiPost<string>('/cluster/action', {
      cluster: selected.cluster,
      namespace: selected.namespace,
      kind: selected.kind,
      name: selected.name,
      action,
      replicas: action === 'scale' ? Number.parseInt(replicasInput, 10) : undefined,
      api_version: selected.kind === 'CustomResource' ? customApiVersion : undefined,
      plural: selected.kind === 'CustomResource' ? customPlural : undefined,
      namespaced: selected.kind === 'CustomResource' ? customNamespaced : undefined,
    });
    setActionLoading('');
    if (!response.success) {
      toast(`${action} failed: ${response.error ?? 'unknown error'}`, 'error');
      return;
    }
    toast(`${action} ${selected.kind} ${selected.name}`, 'success');
    if (action === 'delete') {
      setSelected(null);
    } else {
      await openDetail({
        cluster: selected.cluster,
        namespace: selected.namespace,
        kind: selected.kind,
        name: selected.name,
        status: '',
        created_at: manifestCreationTimestamp(selected.manifest),
        detail: null,
      });
    }
    await loadResources();
  }

  function disconnectExec() {
    execSocketRef.current?.close();
    execSocketRef.current = null;
    setExecConnected(false);
  }

  function connectExec() {
    if (!selected || !execPod) return;
    disconnectExec();
    setExecOutput('');

    const socket = new WebSocket(
      apiWebSocketUrl(
        `/cluster/ws/exec?cluster=${encodeURIComponent(selected.cluster)}&namespace=${encodeURIComponent(selected.namespace)}&pod=${encodeURIComponent(execPod)}&command=${encodeURIComponent(execCommand)}${execContainer ? `&container=${encodeURIComponent(execContainer)}` : ''}`
      )
    );
    execSocketRef.current = socket;

    socket.onopen = () => {
      setExecConnected(true);
      setExecOutput((current) => `${current}[aether] connected to ${execPod}\n`);
    };
    socket.onclose = () => setExecConnected(false);
    socket.onerror = () => {
      setExecConnected(false);
      toast('Terminal connection failed', 'error');
    };
    socket.onmessage = (event) => {
      setExecOutput((current) => `${current}${String(event.data)}`);
    };
  }

  function sendExecLine() {
    if (!execSocketRef.current || execSocketRef.current.readyState !== WebSocket.OPEN || !execInput) return;
    execSocketRef.current.send(`${execInput}\n`);
    setExecInput('');
  }

  async function handlePortForwardStart() {
    if (!selected || (selected.kind !== 'Service' && !portForwardPod)) return;
    setActionLoading('port-forward');
    const response = await apiPost<ClusterPortForwardSession>('/cluster/port-forward', {
      cluster: selected.cluster,
      namespace: selected.namespace,
      target_kind: selected.kind === 'Service' ? 'Service' : 'Pod',
      target_name: selected.kind === 'Service' ? selected.name : portForwardPod,
      pod: selected.kind === 'Service' ? undefined : portForwardPod,
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
    toast(`Forwarding ${response.data.target_name}:${response.data.remote_port} to localhost:${response.data.local_port}`, 'success');
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

  async function handleRolloutAction(action: 'pause' | 'resume' | 'undo' | 'restart') {
    if (!selected) return;
    if (!canMutateCluster) {
      toast(`Your role cannot ${action} rollouts`, 'error');
      return;
    }
    setActionLoading(`rollout-${action}`);
    const response = await apiPost<string>('/cluster/rollout/action', {
      cluster: selected.cluster,
      namespace: selected.namespace,
      kind: selected.kind,
      name: selected.name,
      action,
      revision: action === 'undo' ? rolloutRevision : undefined,
    });
    setActionLoading('');
    if (!response.success) {
      toast(`Rollout ${action} failed: ${response.error ?? 'unknown error'}`, 'error');
      return;
    }
    toast(`Rollout ${action} triggered`, 'success');
    await openDetail({
      cluster: selected.cluster,
      namespace: selected.namespace,
      kind: selected.kind,
      name: selected.name,
      status: '',
      created_at: manifestCreationTimestamp(selected.manifest),
      detail: null,
    });
  }

  async function handleHelmAction(action: 'upgrade' | 'install' | 'rollback') {
    if (!selected) return;
    if (!canMutateCluster) {
      toast(`Your role cannot ${action} Helm releases`, 'error');
      return;
    }
    setActionLoading(`helm-${action}`);
    const response = await apiPost<string>('/cluster/helm/action', {
      cluster: selected.cluster,
      namespace: selected.namespace,
      release: selected.name,
      action,
      chart: action === 'rollback' ? undefined : helmChart,
      values_yaml: action === 'rollback' ? undefined : helmValues,
      revision: action === 'rollback' ? helmRevision : undefined,
    });
    setActionLoading('');
    if (!response.success) {
      toast(`Helm ${action} failed: ${response.error ?? 'unknown error'}`, 'error');
      return;
    }
    toast(`Helm ${action} triggered`, 'success');
    await openDetail({
      cluster: selected.cluster,
      namespace: selected.namespace,
      kind: selected.kind,
      name: selected.name,
      status: '',
      created_at: manifestCreationTimestamp(selected.manifest),
      detail: null,
    });
  }

  async function handleCreateResource() {
    if (!canMutateCluster) {
      toast('Your role cannot create cluster resources', 'error');
      return;
    }
    setActionLoading('create');
    try {
      if (kind === 'HelmRelease') {
        const response = await apiPost<string>('/cluster/helm/action', {
          cluster,
          namespace: namespace === 'all' ? 'default' : namespace,
          release: createHelmRelease,
          action: 'install',
          chart: createHelmChart,
          values_yaml: createHelmValues,
        });
        if (!response.success) {
          throw new Error(response.error ?? 'helm install failed');
        }
        toast(`Installed Helm release ${createHelmRelease}`, 'success');
        setCreateModalOpen(false);
        setCreateHelmRelease('');
        setCreateHelmChart('');
        setCreateHelmValues('');
        await loadResources(cluster, namespace, kind);
        return;
      }

      const manifest = JSON.parse(createManifestDraft);
      const manifestKind = typeof manifest.kind === 'string' ? manifest.kind : kind;
      const manifestNamespace = manifestKind === 'Namespace'
        ? '_cluster'
        : (typeof manifest?.metadata?.namespace === 'string' ? manifest.metadata.namespace : (namespace === 'all' ? 'default' : namespace));
      const response = await apiPost<string>('/cluster/apply', {
        cluster,
        namespace: manifestNamespace,
        kind: manifestKind,
        manifest,
        api_version: manifestKind === 'CustomResource' ? customApiVersion : undefined,
        plural: manifestKind === 'CustomResource' ? customPlural : undefined,
        namespaced: manifestKind === 'CustomResource' ? customNamespaced : undefined,
      });
      if (!response.success) {
        throw new Error(response.error ?? 'create failed');
      }
      toast(`Applied ${manifestKind}`, 'success');
      setCreateModalOpen(false);
      setCreateManifestDraft('');
      await loadResources(cluster, namespace, kind);
    } catch (error) {
      toast(`Create failed: ${String(error)}`, 'error');
    } finally {
      setActionLoading('');
    }
  }

  if (bootstrapLoading) {
    return <PageLoading rows={6} />;
  }

  if (bootstrapFailed) {
    return (
      <PageLoadError
        title="Cluster browser unavailable"
        description="Could not load Kubernetes cluster summary from the API."
        onRetry={() => void loadBootstrap()}
      />
    );
  }

  if (!summary || !summary.enabled) {
    return (
      <EmptyState
        icon={<Container size={48} />}
        title="No Kubernetes contexts"
        description="Aether could not find any kubeconfig contexts to browse."
      />
    );
  }

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard title="Clusters" value={summary.cluster_count} color="blue" />
        <StatCard title="Reachable" value={summary.healthy_clusters} color="green" />
        <StatCard title="Namespaces" value={namespaces.length} color="orange" />
        <StatCard title={pageTab === 'network' ? 'Policies' : `${kind}s`} value={resources.length} color="purple" />
      </div>

      {ciliumStatus && (
        <div className="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-4 py-3 text-sm">
          <span className="text-zinc-500">CNI</span>
          <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${ciliumStatus.cni === 'cilium' ? 'bg-emerald-900/40 text-emerald-300' : 'bg-zinc-800 text-zinc-300'}`}>
            {ciliumStatus.cni}
          </span>
          <span className="text-zinc-600">·</span>
          <span className="text-zinc-500">Egress</span>
          <span className="text-zinc-200">{ciliumStatus.egress_mode}</span>
          <span className="text-zinc-600">·</span>
          <span className="text-zinc-500">metrics-server</span>
          <span className={ciliumStatus.metrics_server ? 'text-emerald-400' : 'text-amber-400'}>
            {ciliumStatus.metrics_server ? 'ok' : 'missing'}
          </span>
        </div>
      )}

      <div data-testid="clusters-page-tabs">
      <PageTabs
        tabs={[
          { id: 'browse', label: 'Browse' },
          { id: 'network', label: 'Network' },
        ]}
        active={pageTab}
        onChange={(tab) => setPageTabWithUrl(tab as 'browse' | 'network')}
      />
      </div>

      {metricsSummary && (
        <div className="grid grid-cols-1 gap-3 lg:grid-cols-3">
          <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
            <div className="text-xs uppercase tracking-wider text-zinc-500">Metrics Scope</div>
            <div className="mt-2 text-lg font-semibold text-zinc-100">{metricsSummary.scope}</div>
          </div>
          <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
            <div className="text-xs uppercase tracking-wider text-zinc-500">CPU</div>
            <div className="mt-2 text-lg font-semibold text-zinc-100">{metricsSummary.total_cpu_millicores}m</div>
            <div className="mt-1 text-xs text-zinc-500">{metricsSummary.pod_count} pods measured</div>
          </div>
          <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
            <div className="text-xs uppercase tracking-wider text-zinc-500">Memory</div>
            <div className="mt-2 text-lg font-semibold text-zinc-100">{metricsSummary.total_memory_mib} Mi</div>
            <div className="mt-1 text-xs text-zinc-500">From `kubectl top pod`</div>
          </div>
        </div>
      )}

      {authStatus && (
        <div className="rounded-xl border border-zinc-800 bg-zinc-900 px-4 py-3 text-sm text-zinc-300">
          Cluster role: <span className="text-zinc-100">{authStatus.role}</span>
          <span className="text-zinc-500"> · </span>
          Signed in as <span className="text-zinc-100">{authStatus.username}</span>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-3">
        <select
          value={cluster}
          onChange={(e) => {
            setCluster(e.target.value);
            setClusterContext(e.target.value);
          }}
          className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
        >
          {summary.clusters.map((item) => (
            <option key={item.name} value={item.name}>
              {item.name} {item.reachable ? '' : '(offline)'}
            </option>
          ))}
        </select>
        <select value={namespace} onChange={(e) => setNamespace(e.target.value)} className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100">
          <option value="all">All namespaces</option>
          {namespaces.map((item) => (
            <option key={item.name} value={item.name}>{item.name}</option>
          ))}
        </select>
        <select value={kind} onChange={(e) => setKind(e.target.value)} className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100" disabled={pageTab === 'network'}>
          {kindOptions.map((item) => (
            <option key={item} value={item}>{item}</option>
          ))}
        </select>
        <div className="rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-400">
          {cluster || 'No cluster selected'} · {watchConnected ? 'watching live' : 'watch offline'}
        </div>
      </div>

      {kind === 'CustomResource' && pageTab === 'browse' && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-3">
          <input
            value={customApiVersion}
            onChange={(e) => setCustomApiVersion(e.target.value)}
            className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
            placeholder="apiVersion e.g. example.com/v1"
          />
          <input
            value={customPlural}
            onChange={(e) => setCustomPlural(e.target.value)}
            className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
            placeholder="plural e.g. widgets"
          />
          <label className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-200">
            <input type="checkbox" checked={customNamespaced} onChange={(e) => setCustomNamespaced(e.target.checked)} />
            Namespaced resource
          </label>
        </div>
      )}

      {pageTab === 'browse' && (
      <div className="flex justify-end">
        <button
          onClick={() => {
            if (kind === 'HelmRelease') {
              setCreateHelmRelease('');
              setCreateHelmChart('');
              setCreateHelmValues('');
            } else {
              setCreateManifestDraft(JSON.stringify({
                apiVersion: defaultApiVersion(kind),
                kind,
                metadata: {
                  name: '',
                  ...(kind !== 'Namespace' && namespace !== 'all' ? { namespace } : {}),
                },
              }, null, 2));
            }
            setCreateModalOpen(true);
          }}
          className="flex items-center gap-2 rounded-lg border border-emerald-600/30 bg-emerald-600/10 px-4 py-2 text-sm text-emerald-300 hover:bg-emerald-600/20"
        >
          <Plus size={16} />
          Create Resource
        </button>
      </div>
      )}

      {loading ? (
        <PageLoading rows={4} />
      ) : resources.length === 0 ? (
        <EmptyState
          icon={<Container size={48} />}
          title="No resources found"
          description={pageTab === 'network' ? 'No NetworkPolicy or Cilium policies in this scope.' : 'Try a different cluster, namespace, or resource kind.'}
        />
      ) : (
        <div className={`dash-card-flush ${panelClass}`}>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Name</th>
                  {pageTab === 'network' && (
                    <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Kind</th>
                  )}
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Namespace</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Status</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Detail</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Created</th>
                </tr>
              </thead>
              <tbody>
                {resources.map((resource) => (
                  <tr key={`${resource.kind}/${resource.namespace}/${resource.name}`} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-4">
                      <div className="flex items-center gap-2">
                        <button onClick={() => openDetail(resource)} className="text-left font-medium text-zinc-200 hover:text-aether transition-colors">
                          {resource.name}
                        </button>
                        {pageTab === 'browse' && ['Deployment', 'StatefulSet', 'DaemonSet'].includes(resource.kind) && (
                          <Link
                            to={pathWithQuery(viewToPath('workloads'), { workload: resource.name, source: 'cluster' })}
                            className="text-xs text-aether hover:underline"
                            title="Open in workloads"
                          >
                            →
                          </Link>
                        )}
                      </div>
                    </td>
                    {pageTab === 'network' && (
                      <td className="py-3 px-4 text-sm text-zinc-400">{resource.kind}</td>
                    )}
                    <td className="py-3 px-4 text-sm text-zinc-400">{resource.namespace}</td>
                    <td className="py-3 px-4 text-sm text-zinc-200">{resource.status}</td>
                    <td className="py-3 px-4 text-sm text-zinc-500">{resource.detail ?? '—'}</td>
                    <td className="py-3 px-4 text-sm text-zinc-400">{formatTimestamp(resource.created_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal
        isOpen={detailLoading || selected !== null}
        onClose={() => {
          disconnectExec();
          if (portForwardSession) {
            void handlePortForwardStop();
          }
          setSelected(null);
          setSelectedEvents([]);
          setHealthSummary(null);
          setRollout(null);
          setDetailTab('overview');
          setDetailLoading(false);
        }}
        title={selected ? `${selected.kind}: ${selected.name}` : 'Loading resource'}
        size="wide"
      >
        {detailLoading || !selected ? (
          <PageLoading label="Loading resource details…" className="h-48" />
        ) : (
          <div className="space-y-4">
            <PageTabs
              tabs={[
                { id: 'overview', label: 'Overview' },
                { id: 'events', label: 'Events' },
                { id: 'logs', label: 'Logs' },
                { id: 'terminal', label: 'Terminal' },
                { id: 'manifest', label: 'Manifest' },
              ]}
              active={detailTab}
              onChange={setDetailTab}
            />
            {detailTab === 'overview' && (
            <div className="flex flex-wrap gap-2">
              {selected.kind === 'Node' && (
                <>
                  <button
                    onClick={() => handleResourceAction('drain')}
                    disabled={!!actionLoading || !canMutateCluster}
                    className="flex items-center gap-2 rounded-lg border border-red-700 bg-red-900/20 px-3 py-2 text-sm text-red-200 hover:bg-red-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'drain' ? 'Draining...' : 'Drain'}
                  </button>
                  <button
                    onClick={() => handleResourceAction('cordon')}
                    disabled={!!actionLoading || !canMutateCluster}
                    className="flex items-center gap-2 rounded-lg border border-amber-700 bg-amber-900/20 px-3 py-2 text-sm text-amber-200 hover:bg-amber-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'cordon' ? 'Cordoning...' : 'Cordon'}
                  </button>
                  <button
                    onClick={() => handleResourceAction('uncordon')}
                    disabled={!!actionLoading || !canMutateCluster}
                    className="flex items-center gap-2 rounded-lg border border-emerald-700 bg-emerald-900/20 px-3 py-2 text-sm text-emerald-200 hover:bg-emerald-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'uncordon' ? 'Uncordoning...' : 'Uncordon'}
                  </button>
                </>
              )}
              {selected.kind === 'VirtualMachine' && (
                <>
                  <button
                    onClick={() => handleResourceAction('start')}
                    disabled={!!actionLoading || !canMutateCluster}
                    className="flex items-center gap-2 rounded-lg border border-emerald-700 bg-emerald-900/20 px-3 py-2 text-sm text-emerald-200 hover:bg-emerald-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'start' ? 'Starting...' : 'Start'}
                  </button>
                  <button
                    onClick={() => handleResourceAction('stop')}
                    disabled={!!actionLoading || !canMutateCluster}
                    className="flex items-center gap-2 rounded-lg border border-amber-700 bg-amber-900/20 px-3 py-2 text-sm text-amber-200 hover:bg-amber-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'stop' ? 'Stopping...' : 'Stop'}
                  </button>
                </>
              )}
              {selected.kind === 'CronJob' && (
                <>
                  <button
                    onClick={() => handleResourceAction('suspend')}
                    disabled={!!actionLoading || !canMutateCluster}
                    className="flex items-center gap-2 rounded-lg border border-amber-700 bg-amber-900/20 px-3 py-2 text-sm text-amber-200 hover:bg-amber-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'suspend' ? 'Suspending...' : 'Suspend'}
                  </button>
                  <button
                    onClick={() => handleResourceAction('resume')}
                    disabled={!!actionLoading || !canMutateCluster}
                    className="flex items-center gap-2 rounded-lg border border-emerald-700 bg-emerald-900/20 px-3 py-2 text-sm text-emerald-200 hover:bg-emerald-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'resume' ? 'Resuming...' : 'Resume'}
                  </button>
                </>
              )}
              {['Deployment', 'StatefulSet'].includes(selected.kind) && (
                <>
                  <div className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2">
                    <span className="text-xs text-zinc-500">Replicas</span>
                    <input
                      type="number"
                      min={0}
                      value={replicasInput}
                      onChange={(e) => setReplicasInput(e.target.value)}
                      className="w-20 rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-sm text-zinc-100"
                    />
                  </div>
                  <button
                    onClick={() => handleResourceAction('scale')}
                    disabled={!!actionLoading || !canMutateCluster}
                    className="flex items-center gap-2 rounded-lg border border-blue-700 bg-blue-900/20 px-3 py-2 text-sm text-blue-200 hover:bg-blue-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'scale' ? 'Scaling...' : 'Scale'}
                  </button>
                </>
              )}
              {['Pod', 'Deployment', 'StatefulSet', 'DaemonSet', 'VirtualMachine', 'VirtualMachineInstance'].includes(selected.kind) && (
                <button
                  onClick={() => handleResourceAction('restart')}
                  disabled={!!actionLoading || !canMutateCluster}
                  className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-200 hover:bg-zinc-700 disabled:opacity-50"
                >
                  <RefreshCw size={14} />
                  {actionLoading === 'restart' ? 'Restarting...' : 'Restart'}
                </button>
              )}
              <button
                onClick={() => handleResourceAction('delete')}
                disabled={!!actionLoading || selected.kind === 'Node' || !canDeleteCluster}
                className="flex items-center gap-2 rounded-lg border border-red-600/30 bg-red-600/10 px-3 py-2 text-sm text-red-300 hover:bg-red-600/20 disabled:opacity-50"
              >
                <Trash2 size={14} />
                {selected.kind === 'Node' ? 'Delete Disabled' : actionLoading === 'delete' ? 'Deleting...' : 'Delete'}
              </button>
            </div>
            )}

            {(detailTab === 'overview' || detailTab === 'manifest') && (
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div><span className="text-zinc-500">Cluster</span><p className="text-white">{selected.cluster}</p></div>
              <div><span className="text-zinc-500">Namespace</span><p className="text-white">{selected.namespace}</p></div>
              <div><span className="text-zinc-500">Kind</span><p className="text-white">{selected.kind}</p></div>
              <div><span className="text-zinc-500">API Version</span><p className="text-white">{selected.api_version ?? 'unknown'}</p></div>
            </div>
            )}

            {detailTab === 'overview' && ((selected.owner_references?.length ?? 0) > 0 || (selected.owned_resources?.length ?? 0) > 0) && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Owner graph</h4>
                {(selected.owner_references?.length ?? 0) > 0 && (
                  <div className="mb-3">
                    <p className="text-xs text-zinc-500 mb-2">Owned by</p>
                    <div className="flex flex-wrap gap-2">
                      {selected.owner_references!.map((owner) => (
                        <span
                          key={`${owner.kind}/${owner.name}`}
                          className="rounded-full border border-zinc-700 bg-zinc-900 px-3 py-1 text-xs text-zinc-200"
                        >
                          {owner.kind}/{owner.name}
                          {owner.controller ? ' (controller)' : ''}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
                {(selected.owned_resources?.length ?? 0) > 0 && (
                  <div>
                    <p className="text-xs text-zinc-500 mb-2">Owns</p>
                    <div className="flex flex-wrap gap-2">
                      {selected.owned_resources!.map((child) => (
                        <span
                          key={`${child.kind}/${child.name}`}
                          className="rounded-full border border-amber-800/40 bg-amber-950/30 px-3 py-1 text-xs text-amber-100"
                        >
                          {child.kind}/{child.name}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}

            {detailTab === 'overview' && healthSummary && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Health</h4>
                <div className="grid grid-cols-1 gap-3 lg:grid-cols-4 text-sm">
                  <div>
                    <div className="text-zinc-500">Level</div>
                    <div className={healthSummary.level === 'healthy' ? 'text-emerald-400' : healthSummary.level === 'degraded' ? 'text-amber-400' : 'text-red-400'}>{healthSummary.level}</div>
                  </div>
                  <div>
                    <div className="text-zinc-500">Pods</div>
                    <div className="text-zinc-100">{healthSummary.ready_pods}/{healthSummary.total_pods} ready</div>
                  </div>
                  <div>
                    <div className="text-zinc-500">Warnings</div>
                    <div className="text-zinc-100">{healthSummary.warning_events}</div>
                  </div>
                  <div>
                    <div className="text-zinc-500">Summary</div>
                    <div className="text-zinc-100">{healthSummary.summary}</div>
                  </div>
                </div>
              </div>
            )}

            {detailTab === 'overview' && selected.conditions.length > 0 && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Conditions</h4>
                <div className="space-y-2">
                  {selected.conditions.map((condition) => (
                    <div key={`${condition.type_}:${condition.reason ?? 'none'}`} className="rounded-md bg-zinc-900 px-3 py-2 text-sm">
                      <div className="flex items-center justify-between">
                        <span className="font-medium text-zinc-100">{condition.type_}</span>
                        <span className={condition.status === 'True' ? 'text-emerald-400' : 'text-amber-400'}>{condition.status}</span>
                      </div>
                      {condition.reason && <div className="mt-1 text-xs text-zinc-400">{condition.reason}</div>}
                      {condition.message && <div className="mt-1 text-xs text-zinc-500">{condition.message}</div>}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {detailTab === 'overview' && selected.pods.length > 0 && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Pods</h4>
                <div className="space-y-2">
                  {selected.pods.map((pod) => (
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

            {detailTab === 'events' && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Event correlation</h4>
                <EventCorrelationPanel events={selectedEvents} resourceName={selected.name} />
              </div>
            )}

            {detailTab === 'events' && selectedEvents.length > 0 && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Cluster events</h4>
                <div className="space-y-2 max-h-64 overflow-auto">
                  {selectedEvents.map((event, index) => (
                    <div key={`${event.timestamp}:${event.reason}:${index}`} className="rounded-md bg-zinc-900 px-3 py-2 text-sm">
                      <div className="flex items-center justify-between gap-3">
                        <div className="text-zinc-100">{event.reason}</div>
                        <div className={event.type_ === 'Warning' ? 'text-amber-400 text-xs' : 'text-emerald-400 text-xs'}>
                          {event.type_}
                        </div>
                      </div>
                      <div className="mt-1 text-xs text-zinc-400">{event.message || 'No event message'}</div>
                      <div className="mt-1 text-xs text-zinc-600">{formatTimestamp(event.timestamp)}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {detailTab === 'overview' && relatedAudit.length > 0 && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Related audit trail</h4>
                <div className="space-y-2 max-h-48 overflow-auto">
                  {relatedAudit.map((entry) => (
                    <div key={entry.id} className="rounded-md bg-zinc-900 px-3 py-2 text-sm">
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-zinc-100">{String(entry.action)}</span>
                        <span className="text-xs text-zinc-500">{String(entry.result)}</span>
                      </div>
                      <div className="mt-1 text-xs text-zinc-400">{entry.message}</div>
                      <div className="mt-1 text-xs text-zinc-600 truncate" title={entry.workload}>
                        {entry.workload}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {detailTab === 'overview' && topMetrics.length > 0 && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Metrics</h4>
                <div className="space-y-2">
                  {topMetrics.map((metric) => (
                    <div key={metric.name} className="grid grid-cols-3 gap-3 rounded-md bg-zinc-900 px-3 py-2 text-sm">
                      <div className="text-zinc-100">{metric.name}</div>
                      <div className="text-zinc-300">CPU {metric.cpu}</div>
                      <div className="text-zinc-300">Memory {metric.memory}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {detailTab === 'overview' && rollout && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <div className="mb-3 flex items-center justify-between">
                  <h4 className="text-sm font-semibold text-zinc-200">Rollout</h4>
                  <div className="text-xs text-zinc-400">{rollout.status}</div>
                </div>
                <div className="mb-3 flex flex-wrap gap-2">
                  <button
                    onClick={() => handleRolloutAction('pause')}
                    disabled={!!actionLoading}
                    className="rounded-lg border border-amber-700 bg-amber-900/20 px-3 py-2 text-sm text-amber-200 hover:bg-amber-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'rollout-pause' ? 'Pausing...' : 'Pause'}
                  </button>
                  <button
                    onClick={() => handleRolloutAction('resume')}
                    disabled={!!actionLoading}
                    className="rounded-lg border border-emerald-700 bg-emerald-900/20 px-3 py-2 text-sm text-emerald-200 hover:bg-emerald-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'rollout-resume' ? 'Resuming...' : 'Resume'}
                  </button>
                  <button
                    onClick={() => handleRolloutAction('restart')}
                    disabled={!!actionLoading}
                    className="rounded-lg border border-blue-700 bg-blue-900/20 px-3 py-2 text-sm text-blue-200 hover:bg-blue-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'rollout-restart' ? 'Restarting...' : 'Rollout Restart'}
                  </button>
                  <div className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2">
                    <span className="text-xs text-zinc-500">Revision</span>
                    <select value={rolloutRevision} onChange={(e) => setRolloutRevision(e.target.value)} className="bg-transparent text-sm text-zinc-100 outline-none">
                      {rollout.history.map((entry) => (
                        <option key={entry.revision} value={entry.revision}>
                          {entry.revision}
                        </option>
                      ))}
                    </select>
                    <button
                      onClick={() => handleRolloutAction('undo')}
                      disabled={!!actionLoading || !rolloutRevision}
                      className="rounded-lg border border-zinc-600 bg-zinc-800 px-3 py-1 text-sm text-zinc-200 hover:bg-zinc-700 disabled:opacity-50"
                    >
                      {actionLoading === 'rollout-undo' ? 'Undoing...' : 'Undo'}
                    </button>
                  </div>
                </div>
                <div className="space-y-2">
                  {rollout.history.length === 0 ? (
                    <div className="text-sm text-zinc-500">No rollout revisions reported.</div>
                  ) : (
                    rollout.history.map((entry) => (
                      <div key={entry.revision} className="rounded-md bg-zinc-900 px-3 py-2 text-sm">
                        <div className="text-zinc-100">Revision {entry.revision}</div>
                        <div className="mt-1 text-xs text-zinc-500">{entry.change_cause}</div>
                      </div>
                    ))
                  )}
                </div>
              </div>
            )}

            {detailTab === 'overview' && selected.kind === 'HelmRelease' && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <div className="mb-3 flex items-center justify-between">
                  <h4 className="text-sm font-semibold text-zinc-200">Helm</h4>
                  <div className="text-xs text-zinc-400">{helmHistory.length} revisions</div>
                </div>
                <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
                  <input
                    value={helmChart}
                    onChange={(e) => setHelmChart(e.target.value)}
                    className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
                    placeholder="repo/chart or chart reference"
                  />
                  <div className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2">
                    <span className="text-xs text-zinc-500">Revision</span>
                    <select value={helmRevision} onChange={(e) => setHelmRevision(e.target.value)} className="bg-transparent text-sm text-zinc-100 outline-none">
                      {helmHistory.map((entry) => (
                        <option key={entry.revision} value={entry.revision}>{entry.revision}</option>
                      ))}
                    </select>
                  </div>
                </div>
                <textarea
                  value={helmValues}
                  onChange={(e) => setHelmValues(e.target.value)}
                  rows={10}
                  className="mt-3 w-full rounded bg-zinc-950 p-3 text-xs text-zinc-300 font-mono border border-zinc-800"
                  spellCheck={false}
                  placeholder="Helm values YAML"
                />
                <div className="mt-3 flex flex-wrap gap-2">
                  <button
                    onClick={() => handleHelmAction('upgrade')}
                    disabled={!!actionLoading || !helmChart}
                    className="rounded-lg border border-blue-700 bg-blue-900/20 px-3 py-2 text-sm text-blue-200 hover:bg-blue-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'helm-upgrade' ? 'Upgrading...' : 'Upgrade'}
                  </button>
                  <button
                    onClick={() => handleHelmAction('install')}
                    disabled={!!actionLoading || !helmChart}
                    className="rounded-lg border border-emerald-700 bg-emerald-900/20 px-3 py-2 text-sm text-emerald-200 hover:bg-emerald-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'helm-install' ? 'Installing...' : 'Install'}
                  </button>
                  <button
                    onClick={() => handleHelmAction('rollback')}
                    disabled={!!actionLoading || !helmRevision}
                    className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-200 hover:bg-zinc-800 disabled:opacity-50"
                  >
                    {actionLoading === 'helm-rollback' ? 'Rolling Back...' : 'Rollback'}
                  </button>
                </div>
                <div className="mt-3 space-y-2">
                  {helmHistory.map((entry) => (
                    <div key={entry.revision} className="rounded-md bg-zinc-900 px-3 py-2 text-sm">
                      <div className="flex items-center justify-between">
                        <span className="text-zinc-100">Revision {entry.revision}</span>
                        <span className="text-zinc-400">{entry.status}</span>
                      </div>
                      <div className="mt-1 text-xs text-zinc-500">{entry.chart}{entry.app_version ? ` · ${entry.app_version}` : ''}</div>
                      <div className="mt-1 text-xs text-zinc-600">{entry.description ?? 'No description'} · {formatTimestamp(entry.updated)}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {detailTab === 'logs' && selectedLogsPath && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Logs</h4>
                <LogViewer workloadName={selected.name} logsPath={selectedLogsPath} />
              </div>
            )}

            {detailTab === 'terminal' && ((selected.kind === 'Pod' && selected.name) || selected.pods.length > 0) && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <div className="mb-3 flex items-center justify-between">
                  <h4 className="text-sm font-semibold text-zinc-200">Terminal</h4>
                  <div className="flex items-center gap-3">
                    <button
                      onClick={() => setTerminalExpanded((value) => !value)}
                      className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-1 text-xs text-zinc-200 hover:bg-zinc-800"
                    >
                      {terminalExpanded ? 'Compact' : 'Expand'}
                    </button>
                    <div className={`text-xs ${execConnected ? 'text-emerald-400' : 'text-zinc-500'}`}>
                      {execConnected ? 'connected' : 'disconnected'}
                    </div>
                  </div>
                </div>
                <div className="grid grid-cols-1 gap-3 lg:grid-cols-3">
                  <select value={execPod} onChange={(e) => setExecPod(e.target.value)} className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100">
                    {(selected.kind === 'Pod' ? [selected.name] : selected.pods.map((pod) => pod.name)).map((podName) => (
                      <option key={podName} value={podName}>{podName}</option>
                    ))}
                  </select>
                  <select value={execContainer} onChange={(e) => setExecContainer(e.target.value)} disabled={selectedContainers.length === 0} className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 disabled:opacity-60">
                    <option value="">default container</option>
                    {selectedContainers.map((containerName) => (
                      <option key={containerName} value={containerName}>{containerName}</option>
                    ))}
                  </select>
                  <input
                    value={execCommand}
                    onChange={(e) => setExecCommand(e.target.value)}
                    className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
                    placeholder="/bin/sh"
                  />
                  <div className="flex gap-2 lg:col-span-3">
                    <button
                      onClick={connectExec}
                      disabled={!execPod}
                      className="flex-1 rounded-lg border border-blue-700 bg-blue-900/20 px-3 py-2 text-sm text-blue-200 hover:bg-blue-800/30 disabled:opacity-50"
                    >
                      Connect
                    </button>
                    <button
                      onClick={disconnectExec}
                      disabled={!execConnected}
                      className="flex-1 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-200 hover:bg-zinc-800 disabled:opacity-50"
                    >
                      Disconnect
                    </button>
                  </div>
                </div>
                <pre className={`mt-3 overflow-auto rounded-lg border border-zinc-800 bg-black p-3 text-xs text-emerald-300 ${terminalExpanded ? 'h-[65vh]' : 'h-64'}`}>{execOutput || '[aether] terminal idle'}</pre>
                <div className="mt-3 flex gap-2">
                  <input
                    value={execInput}
                    onChange={(e) => setExecInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        e.preventDefault();
                        sendExecLine();
                      }
                    }}
                    className="flex-1 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
                    placeholder="Type a command and press Enter"
                  />
                  <button
                    onClick={sendExecLine}
                    disabled={!execConnected || !execInput}
                    className="rounded-lg border border-emerald-700 bg-emerald-900/20 px-4 py-2 text-sm text-emerald-200 hover:bg-emerald-800/30 disabled:opacity-50"
                  >
                    Send
                  </button>
                </div>
              </div>
            )}

            {detailTab === 'terminal' && (((selected.kind === 'Pod' && selected.name) || selected.pods.length > 0) || selected.kind === 'Service') && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Port Forward</h4>
                <div className="grid grid-cols-1 gap-3 lg:grid-cols-4">
                  <select value={selected.kind === 'Service' ? selected.name : portForwardPod} onChange={(e) => setPortForwardPod(e.target.value)} disabled={selected.kind === 'Service'} className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 disabled:opacity-60">
                    {((selected.kind === 'Service') ? [selected.name] : (selected.kind === 'Pod' ? [selected.name] : selected.pods.map((pod) => pod.name))).map((podName) => (
                      <option key={podName} value={podName}>{podName}</option>
                    ))}
                  </select>
                  <input
                    type="number"
                    value={portForwardRemotePort}
                    onChange={(e) => setPortForwardRemotePort(e.target.value)}
                    className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
                    placeholder="Remote port"
                  />
                  <input
                    type="number"
                    value={portForwardLocalPort}
                    onChange={(e) => setPortForwardLocalPort(e.target.value)}
                    className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
                    placeholder="Local port (optional)"
                  />
                  <div className="flex gap-2">
                    <button
                      onClick={handlePortForwardStart}
                      disabled={!portForwardPod || !portForwardRemotePort || !!portForwardSession}
                      className="flex-1 rounded-lg border border-blue-700 bg-blue-900/20 px-3 py-2 text-sm text-blue-200 hover:bg-blue-800/30 disabled:opacity-50"
                    >
                      {actionLoading === 'port-forward' ? 'Starting...' : 'Start'}
                    </button>
                    <button
                      onClick={handlePortForwardStop}
                      disabled={!portForwardSession}
                      className="flex-1 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-200 hover:bg-zinc-800 disabled:opacity-50"
                    >
                      {actionLoading === 'port-forward-stop' ? 'Stopping...' : 'Stop'}
                    </button>
                  </div>
                </div>
                {portForwardSession && (
                  <div className="mt-3 rounded-md bg-zinc-900 px-3 py-2 text-sm text-zinc-300">
                    Active: <span className="text-emerald-300">{portForwardSession.local_url}</span> {'->'} {portForwardSession.target_kind}/{portForwardSession.target_name}:{portForwardSession.remote_port}
                  </div>
                )}
              </div>
            )}

            {detailTab === 'manifest' && (
            <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
              <div className="mb-3 flex items-center justify-between">
                <h4 className="text-sm font-semibold text-zinc-200">Manifest</h4>
                <button
                  onClick={handleApply}
                  disabled={!!actionLoading || selected.kind === 'HelmRelease'}
                  className="flex items-center gap-2 rounded-lg border border-emerald-600/30 bg-emerald-600/10 px-3 py-2 text-sm text-emerald-300 hover:bg-emerald-600/20 disabled:opacity-50"
                >
                  <Save size={14} />
                  {actionLoading === 'apply' ? 'Applying...' : selected.kind === 'HelmRelease' ? 'Managed by Helm' : 'Apply Changes'}
                </button>
              </div>
              <textarea
                value={manifestDraft}
                onChange={(e) => setManifestDraft(e.target.value)}
                rows={18}
                className="w-full rounded bg-zinc-950 p-3 text-xs text-zinc-300 font-mono border border-zinc-800"
                spellCheck={false}
              />
              <div className="mt-3">
                <CodeBlock title="Current Resource">{JSON.stringify(selected.manifest, null, 2)}</CodeBlock>
              </div>
              <div className="mt-3 rounded-lg border border-zinc-800 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Diff Preview</h4>
                <pre className="max-h-80 overflow-auto text-xs">
                  {serverDiff.map((line, index) => (
                    <div
                      key={`${line.kind}:${index}`}
                      className={
                        line.kind === 'add'
                          ? 'text-emerald-300'
                          : line.kind === 'remove'
                            ? 'text-red-300'
                            : 'text-zinc-500'
                      }
                    >
                      {line.text}
                    </div>
                  ))}
                </pre>
              </div>
            </div>
            )}
          </div>
        )}
      </Modal>

      <Modal
        isOpen={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        title={`Create ${kind}`}
      >
        <div className="space-y-4">
          {kind === 'HelmRelease' ? (
            <>
              <p className="text-sm text-zinc-400">
                Install a Helm release into the selected cluster and namespace.
              </p>
              <input
                value={createHelmRelease}
                onChange={(e) => setCreateHelmRelease(e.target.value)}
                className="w-full rounded border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
                placeholder="release name"
              />
              <input
                value={createHelmChart}
                onChange={(e) => setCreateHelmChart(e.target.value)}
                className="w-full rounded border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
                placeholder="repo/chart or chart path"
              />
              <textarea
                value={createHelmValues}
                onChange={(e) => setCreateHelmValues(e.target.value)}
                rows={14}
                className="w-full rounded bg-zinc-950 p-3 text-xs text-zinc-300 font-mono border border-zinc-800"
                spellCheck={false}
                placeholder="Helm values YAML"
              />
            </>
          ) : (
            <>
              <p className="text-sm text-zinc-400">
                Provide a JSON manifest. Aether will apply it directly to the selected cluster.
              </p>
              <textarea
                value={createManifestDraft}
                onChange={(e) => setCreateManifestDraft(e.target.value)}
                rows={18}
                className="w-full rounded bg-zinc-950 p-3 text-xs text-zinc-300 font-mono border border-zinc-800"
                spellCheck={false}
              />
            </>
          )}
          <div className="flex justify-end">
            <button
              onClick={handleCreateResource}
              disabled={!!actionLoading || (kind === 'HelmRelease' && (!createHelmRelease || !createHelmChart))}
              className="flex items-center gap-2 rounded-lg border border-emerald-600/30 bg-emerald-600/10 px-4 py-2 text-sm text-emerald-300 hover:bg-emerald-600/20 disabled:opacity-50"
            >
              <Save size={14} />
              {actionLoading === 'create' ? (kind === 'HelmRelease' ? 'Installing...' : 'Applying...') : (kind === 'HelmRelease' ? 'Install Release' : 'Create / Apply')}
            </button>
          </div>
        </div>
      </Modal>
    </div>
  );
}

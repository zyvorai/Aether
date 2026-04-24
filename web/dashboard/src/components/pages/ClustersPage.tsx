import { useEffect, useMemo, useState } from 'react';
import { Container, Plus, RefreshCw, Save, Trash2 } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import Modal from '../Modal';
import LogViewer from '../LogViewer';
import EmptyState from '../EmptyState';
import StatCard from '../StatCard';
import CodeBlock from '../CodeBlock';
import { formatTimestamp } from '../../utils/formatters';
import type {
  ClusterBrowseItem,
  ClusterNamespaceSummary,
  ClusterResourceDetail,
  ClusterSummary,
} from '../../types/api';

const kindOptions = ['Namespace', 'Pod', 'ServiceAccount', 'Secret', 'PersistentVolumeClaim', 'Deployment', 'StatefulSet', 'DaemonSet', 'Job', 'CronJob', 'HorizontalPodAutoscaler', 'Service', 'Ingress', 'NetworkPolicy', 'ConfigMap', 'Event', 'DataVolume', 'VirtualMachine', 'VirtualMachineInstance'];

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
  if (['VirtualMachine', 'VirtualMachineInstance'].includes(kind)) return 'kubevirt.io/v1';
  if (kind === 'DataVolume') return 'cdi.kubevirt.io/v1beta1';
  return 'v1';
}

export default function ClustersPage() {
  const [summary, setSummary] = useState<ClusterSummary | null>(null);
  const [cluster, setCluster] = useState('');
  const [namespace, setNamespace] = useState('all');
  const [kind, setKind] = useState('Pod');
  const [namespaces, setNamespaces] = useState<ClusterNamespaceSummary[]>([]);
  const [resources, setResources] = useState<ClusterBrowseItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<ClusterResourceDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [manifestDraft, setManifestDraft] = useState('');
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [createManifestDraft, setCreateManifestDraft] = useState('');
  const [actionLoading, setActionLoading] = useState('');
  const [replicasInput, setReplicasInput] = useState('1');

  function toast(message: string, type: 'success' | 'error') {
    window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
  }

  async function loadResources(targetCluster = cluster, targetNamespace = namespace, targetKind = kind) {
    if (!targetCluster) return;
    setLoading(true);
    const data = await apiFetch<ClusterBrowseItem[]>(
      `/cluster/browse?cluster=${encodeURIComponent(targetCluster)}&namespace=${encodeURIComponent(targetNamespace)}&kind=${encodeURIComponent(targetKind)}`
    );
    setResources(data ?? []);
    setLoading(false);
  }

  useEffect(() => {
    apiFetch<ClusterSummary>('/cluster/summary').then((data) => {
      setSummary(data);
      const firstCluster = data?.clusters.find((item) => item.reachable)?.name ?? data?.clusters[0]?.name ?? '';
      setCluster(firstCluster);
      setLoading(false);
    });
  }, []);

  useEffect(() => {
    if (!cluster) return;
    apiFetch<ClusterNamespaceSummary[]>(`/cluster/namespaces?cluster=${encodeURIComponent(cluster)}`).then((data) => {
      setNamespaces(data ?? []);
    });
  }, [cluster]);

  useEffect(() => {
    loadResources();
  }, [cluster, namespace, kind]);

  const selectedLogsPath = useMemo(() => {
    if (!selected) return undefined;
    if (!['Pod', 'Deployment', 'StatefulSet', 'DaemonSet', 'Service', 'VirtualMachine', 'VirtualMachineInstance'].includes(selected.kind)) return undefined;
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
    const detail = await apiFetch<ClusterResourceDetail>(
      `/cluster/resource?cluster=${encodeURIComponent(resource.cluster)}&namespace=${encodeURIComponent(resource.namespace)}&kind=${encodeURIComponent(resource.kind)}&name=${encodeURIComponent(resource.name)}`
    );
    setSelected(detail);
    if (detail) {
      setManifestDraft(JSON.stringify(detail.manifest, null, 2));
      const replicas = (detail.manifest?.spec as { replicas?: number } | undefined)?.replicas;
      if (typeof replicas === 'number') {
        setReplicasInput(String(replicas));
      }
    }
    setDetailLoading(false);
  }

  async function handleApply() {
    if (!selected) return;
    setActionLoading('apply');
    try {
      const manifest = JSON.parse(manifestDraft);
      const response = await apiPost<string>('/cluster/apply', {
        cluster: selected.cluster,
        namespace: selected.namespace,
        kind: selected.kind,
        manifest,
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

  async function handleResourceAction(action: 'start' | 'stop' | 'restart' | 'delete' | 'scale' | 'suspend' | 'resume') {
    if (!selected) return;
    setActionLoading(action);
    const response = await apiPost<string>('/cluster/action', {
      cluster: selected.cluster,
      namespace: selected.namespace,
      kind: selected.kind,
      name: selected.name,
      action,
      replicas: action === 'scale' ? Number.parseInt(replicasInput, 10) : undefined,
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

  async function handleCreateResource() {
    setActionLoading('create');
    try {
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
        <StatCard title={`${kind}s`} value={resources.length} color="purple" />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-3">
        <select value={cluster} onChange={(e) => setCluster(e.target.value)} className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100">
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
        <select value={kind} onChange={(e) => setKind(e.target.value)} className="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100">
          {kindOptions.map((item) => (
            <option key={item} value={item}>{item}</option>
          ))}
        </select>
        <div className="rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-400">
          {cluster || 'No cluster selected'}
        </div>
      </div>

      <div className="flex justify-end">
        <button
          onClick={() => {
            setCreateManifestDraft(JSON.stringify({
              apiVersion: defaultApiVersion(kind),
              kind,
              metadata: {
                name: '',
                ...(kind !== 'Namespace' && namespace !== 'all' ? { namespace } : {}),
              },
            }, null, 2));
            setCreateModalOpen(true);
          }}
          className="flex items-center gap-2 rounded-lg border border-emerald-600/30 bg-emerald-600/10 px-4 py-2 text-sm text-emerald-300 hover:bg-emerald-600/20"
        >
          <Plus size={16} />
          Create Resource
        </button>
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-48">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
        </div>
      ) : resources.length === 0 ? (
        <EmptyState
          icon={<Container size={48} />}
          title="No resources found"
          description="Try a different cluster, namespace, or resource kind."
        />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Namespace</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Status</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Detail</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Created</th>
                </tr>
              </thead>
              <tbody>
                {resources.map((resource) => (
                  <tr key={`${resource.namespace}/${resource.name}`} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-4">
                      <button onClick={() => openDetail(resource)} className="text-left font-medium text-zinc-200 hover:text-aether transition-colors">
                        {resource.name}
                      </button>
                    </td>
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
          setSelected(null);
          setDetailLoading(false);
        }}
        title={selected ? `${selected.kind}: ${selected.name}` : 'Loading resource'}
      >
        {detailLoading || !selected ? (
          <div className="flex items-center justify-center h-48">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex flex-wrap gap-2">
              {selected.kind === 'VirtualMachine' && (
                <>
                  <button
                    onClick={() => handleResourceAction('start')}
                    disabled={!!actionLoading}
                    className="flex items-center gap-2 rounded-lg border border-emerald-700 bg-emerald-900/20 px-3 py-2 text-sm text-emerald-200 hover:bg-emerald-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'start' ? 'Starting...' : 'Start'}
                  </button>
                  <button
                    onClick={() => handleResourceAction('stop')}
                    disabled={!!actionLoading}
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
                    disabled={!!actionLoading}
                    className="flex items-center gap-2 rounded-lg border border-amber-700 bg-amber-900/20 px-3 py-2 text-sm text-amber-200 hover:bg-amber-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'suspend' ? 'Suspending...' : 'Suspend'}
                  </button>
                  <button
                    onClick={() => handleResourceAction('resume')}
                    disabled={!!actionLoading}
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
                    disabled={!!actionLoading}
                    className="flex items-center gap-2 rounded-lg border border-blue-700 bg-blue-900/20 px-3 py-2 text-sm text-blue-200 hover:bg-blue-800/30 disabled:opacity-50"
                  >
                    {actionLoading === 'scale' ? 'Scaling...' : 'Scale'}
                  </button>
                </>
              )}
              {['Pod', 'Deployment', 'StatefulSet', 'DaemonSet', 'VirtualMachine', 'VirtualMachineInstance'].includes(selected.kind) && (
                <button
                  onClick={() => handleResourceAction('restart')}
                  disabled={!!actionLoading}
                  className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-200 hover:bg-zinc-700 disabled:opacity-50"
                >
                  <RefreshCw size={14} />
                  {actionLoading === 'restart' ? 'Restarting...' : 'Restart'}
                </button>
              )}
              <button
                onClick={() => handleResourceAction('delete')}
                disabled={!!actionLoading}
                className="flex items-center gap-2 rounded-lg border border-red-600/30 bg-red-600/10 px-3 py-2 text-sm text-red-300 hover:bg-red-600/20 disabled:opacity-50"
              >
                <Trash2 size={14} />
                {actionLoading === 'delete' ? 'Deleting...' : 'Delete'}
              </button>
            </div>

            <div className="grid grid-cols-2 gap-3 text-sm">
              <div><span className="text-zinc-500">Cluster</span><p className="text-white">{selected.cluster}</p></div>
              <div><span className="text-zinc-500">Namespace</span><p className="text-white">{selected.namespace}</p></div>
              <div><span className="text-zinc-500">Kind</span><p className="text-white">{selected.kind}</p></div>
              <div><span className="text-zinc-500">API Version</span><p className="text-white">{selected.api_version ?? 'unknown'}</p></div>
            </div>

            {selected.conditions.length > 0 && (
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

            {selected.pods.length > 0 && (
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

            {selectedLogsPath && (
              <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                <h4 className="mb-3 text-sm font-semibold text-zinc-200">Logs</h4>
                <LogViewer workloadName={selected.name} logsPath={selectedLogsPath} />
              </div>
            )}

            <div className="rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
              <div className="mb-3 flex items-center justify-between">
                <h4 className="text-sm font-semibold text-zinc-200">Manifest</h4>
                <button
                  onClick={handleApply}
                  disabled={!!actionLoading}
                  className="flex items-center gap-2 rounded-lg border border-emerald-600/30 bg-emerald-600/10 px-3 py-2 text-sm text-emerald-300 hover:bg-emerald-600/20 disabled:opacity-50"
                >
                  <Save size={14} />
                  {actionLoading === 'apply' ? 'Applying...' : 'Apply Changes'}
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
            </div>
          </div>
        )}
      </Modal>

      <Modal
        isOpen={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        title={`Create ${kind}`}
      >
        <div className="space-y-4">
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
          <div className="flex justify-end">
            <button
              onClick={handleCreateResource}
              disabled={!!actionLoading}
              className="flex items-center gap-2 rounded-lg border border-emerald-600/30 bg-emerald-600/10 px-4 py-2 text-sm text-emerald-300 hover:bg-emerald-600/20 disabled:opacity-50"
            >
              <Save size={14} />
              {actionLoading === 'create' ? 'Applying...' : 'Create / Apply'}
            </button>
          </div>
        </div>
      </Modal>
    </div>
  );
}

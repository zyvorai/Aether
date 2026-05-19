import { useState, useEffect } from 'react';
import { Play, Square, ArrowRightLeft, Trash2, FileText, Cpu, Search, ClipboardCheck, RefreshCw, Inbox, Hammer, Plus } from 'lucide-react';
import { apiFetch, apiPost, apiDelete } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import Badge, { RuntimeBadge } from '../Badge';
import StatCard from '../StatCard';
import Modal from '../Modal';
import YamlInput from '../YamlInput';
import EmptyState from '../EmptyState';
import WorkloadDetail, { type DetailTab } from '../WorkloadDetail';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import type { WorkloadResponse, ValidateResponse, BuildResponse } from '../../types/api';

interface WorkloadsPageProps {
  initialSelectedName?: string | null;
  onClearInitialSelection?: () => void;
}

function getStatusVariant(status: string): 'green' | 'red' | 'yellow' | 'muted' {
  const s = status.toLowerCase();
  if (s === 'running' || s === 'healthy') return 'green';
  if (s === 'error' || s === 'failed') return 'red';
  if (s === 'stopped' || s === 'exited') return 'muted';
  return 'yellow';
}

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

function isAetherManaged(workload: WorkloadResponse): boolean {
  return (workload.source ?? 'aether') === 'aether';
}

export default function WorkloadsPage({ initialSelectedName, onClearInitialSelection }: WorkloadsPageProps) {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [sourceFilter, setSourceFilter] = useState<'all' | 'aether' | 'cluster'>('all');
  const [kindFilter, setKindFilter] = useState('all');
  const [clusterFilter, setClusterFilter] = useState('all');
  const [namespaceFilter, setNamespaceFilter] = useState('all');
  const [migrateModal, setMigrateModal] = useState<string | null>(null);
  const [validateModal, setValidateModal] = useState(false);
  const [validateResult, setValidateResult] = useState<ValidateResponse | null>(null);
  const [validateLoading, setValidateLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [confirmAction, setConfirmAction] = useState<{ type: 'delete' | 'stop'; name: string } | null>(null);
  const [deployModal, setDeployModal] = useState(false);
  const [deployLoading, setDeployLoading] = useState(false);
  const [selectedWorkload, setSelectedWorkload] = useState<WorkloadResponse | null>(null);
  const [detailInitialTab, setDetailInitialTab] = useState<DetailTab>('overview');

  const runtimes = ['podman', 'docker', 'kubernetes', 'kubevirt', 'metal3'];

  async function load() {
    setLoading(true);
    const data = await apiFetch<WorkloadResponse[]>('/workloads');
    setWorkloads(data ?? []);
    setLoading(false);
  }

  useEffect(() => { load(); }, []);

  useEffect(() => {
    if (!initialSelectedName || workloads.length === 0) return;
    const match = workloads.find((w) => w.name === initialSelectedName);
    if (match) {
      setDetailInitialTab('overview');
      setSelectedWorkload(match);
    }
    onClearInitialSelection?.();
  }, [initialSelectedName, workloads, onClearInitialSelection]);

  function openWorkloadDetail(workload: WorkloadResponse, tab: DetailTab) {
    setDetailInitialTab(tab);
    setSelectedWorkload(workload);
  }

  async function handleAction(name: string, action: string) {
    setActionLoading(`${name}-${action}`);
    let res;
    if (action === 'start') {
      res = await apiPost(`/workloads/${name}/start`);
    } else if (action === 'stop') {
      res = await apiPost(`/workloads/${name}/stop`);
    } else if (action === 'delete') {
      res = await apiDelete(`/workloads/${name}`, { label: `Delete workload "${name}"` });
    }
    setActionLoading(null);
    if (res?.success) {
      toast(`${action.charAt(0).toUpperCase() + action.slice(1)} "${name}" succeeded`, 'success');
    } else {
      toast(`${action.charAt(0).toUpperCase() + action.slice(1)} "${name}" failed: ${res?.error ?? 'unknown error'}`, 'error');
    }
    load();
  }

  async function handleBuild(name: string) {
    setActionLoading(`${name}-build`);
    const res = await apiPost<BuildResponse>(`/workloads/${name}/build`);
    setActionLoading(null);
    if (res.success && res.data) {
      toast(`Build "${name}" succeeded: ${res.data.full_name}`, 'success');
    } else {
      toast(`Build "${name}" failed: ${res.error ?? 'unknown error'}`, 'error');
    }
    load();
  }

  async function handleMigrate(name: string, target: string) {
    setActionLoading(`${name}-migrate`);
    const res = await apiPost(`/workloads/${name}/migrate`, { target_runtime: target });
    setMigrateModal(null);
    setActionLoading(null);
    if (res.success) {
      toast(`Migration of "${name}" to ${target} started`, 'success');
    } else {
      toast(`Migration of "${name}" failed: ${res.error ?? 'unknown error'}`, 'error');
    }
    load();
  }

  async function handleValidate(yaml: string) {
    setValidateLoading(true);
    const result = await apiPost<ValidateResponse>('/validate', { yaml });
    setValidateResult(result.data ?? null);
    setValidateLoading(false);
  }

  async function handleDeploy(yaml: string) {
    setDeployLoading(true);
    const res = await apiPost('/workloads', { spec_yaml: yaml });
    setDeployLoading(false);
    if (res.success) {
      toast('Workload deployed successfully', 'success');
      setDeployModal(false);
      load();
    } else {
      toast(`Deploy failed: ${res.error ?? 'unknown error'}`, 'error');
    }
  }

  function handleConfirmedAction() {
    if (!confirmAction) return;
    handleAction(confirmAction.name, confirmAction.type);
    setConfirmAction(null);
  }

  if (loading) {
    return <PageLoading rows={6} />;
  }

  const kinds = ['all', ...Array.from(new Set(workloads.map((w) => w.kind).filter((kind): kind is string => Boolean(kind)))).sort()];
  const clusters = ['all', ...Array.from(new Set(workloads.map((w) => w.cluster).filter((cluster): cluster is string => Boolean(cluster)))).sort()];
  const namespaces = ['all', ...Array.from(new Set(workloads.map((w) => w.namespace).filter((namespace): namespace is string => Boolean(namespace)))).sort()];

  const filteredWorkloads = workloads.filter((workload) => {
    const matchesSearch = !search || [
      workload.name,
      workload.image,
      workload.runtime,
      workload.cluster ?? '',
      workload.namespace ?? '',
      workload.kind ?? '',
    ].some((value) => value.toLowerCase().includes(search.toLowerCase()));
    const source = workload.source ?? 'aether';
    const matchesSource = sourceFilter === 'all' || source === sourceFilter;
    const matchesKind = kindFilter === 'all' || workload.kind === kindFilter;
    const matchesCluster = clusterFilter === 'all' || workload.cluster === clusterFilter;
    const matchesNamespace = namespaceFilter === 'all' || workload.namespace === namespaceFilter;
    return matchesSearch && matchesSource && matchesKind && matchesCluster && matchesNamespace;
  });

  const filterSelectClass = 'rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100';

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search name, image, cluster, namespace..."
        onRefresh={load}
        refreshing={loading}
        filters={
          <>
            <select value={sourceFilter} onChange={(e) => setSourceFilter(e.target.value as 'all' | 'aether' | 'cluster')} className={filterSelectClass}>
              <option value="all">All sources</option>
              <option value="aether">Aether managed</option>
              <option value="cluster">Kubernetes discovered</option>
            </select>
            <select value={kindFilter} onChange={(e) => setKindFilter(e.target.value)} className={filterSelectClass}>
              {kinds.map((kind) => <option key={kind} value={kind}>{kind === 'all' ? 'All kinds' : kind}</option>)}
            </select>
            <select value={clusterFilter} onChange={(e) => setClusterFilter(e.target.value)} className={filterSelectClass}>
              {clusters.map((cluster) => <option key={cluster} value={cluster}>{cluster === 'all' ? 'All clusters' : cluster}</option>)}
            </select>
            <select value={namespaceFilter} onChange={(e) => setNamespaceFilter(e.target.value)} className={filterSelectClass}>
              {namespaces.map((namespace) => <option key={namespace} value={namespace}>{namespace === 'all' ? 'All namespaces' : namespace}</option>)}
            </select>
          </>
        }
        actions={
          <>
            <button
              type="button"
              onClick={() => setDeployModal(true)}
              className="inline-flex items-center gap-2 rounded-xl bg-aether px-3 py-2 text-sm font-medium text-white transition hover:bg-aether-light"
            >
              <Plus size={16} />
              Deploy
            </button>
            <button
              type="button"
              onClick={() => setValidateModal(true)}
              className="inline-flex items-center gap-2 rounded-xl bg-blue-600 px-3 py-2 text-sm font-medium text-white transition hover:bg-blue-500"
            >
              <ClipboardCheck size={16} />
              Validate
            </button>
          </>
        }
      />

      <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
        <StatCard title="Total" value={filteredWorkloads.length} color="orange" />
        <StatCard
          title="Running"
          value={filteredWorkloads.filter((w) => w.status.toLowerCase() === 'running').length}
          color="green"
        />
        <StatCard
          title="Stopped"
          value={filteredWorkloads.filter((w) => ['stopped', 'exited'].includes(w.status.toLowerCase())).length}
          color="red"
        />
        <StatCard
          title="Aether"
          value={filteredWorkloads.filter((w) => (w.source ?? 'aether') === 'aether').length}
          color="purple"
        />
        <StatCard
          title="Discovered"
          value={filteredWorkloads.filter((w) => (w.source ?? 'aether') === 'cluster').length}
          color="blue"
        />
        <StatCard
          title="Namespaces"
          value={namespaces.filter((namespace) => namespace !== 'all').length}
          color="green"
        />
      </div>

      {filteredWorkloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No workloads" description="Deploy a workload to see it here" />
      ) : (
        <div className="dash-card-flush">
          <div className="overflow-x-auto min-w-0">
            <table className="w-full table-fixed border-collapse">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[18%] min-w-0">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[12%] min-w-0">Runtime</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[36%] min-w-0">Image</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[10%] min-w-0">Status</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[14%] min-w-0">Created</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[10%] min-w-0">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredWorkloads.map((w) => (
                  <tr key={w.name} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-3 sm:px-4 font-medium text-zinc-200 align-top min-w-0">
                      <button
                        type="button"
                        onClick={() => openWorkloadDetail(w, 'overview')}
                        className="hover:text-aether transition-colors text-left w-full min-w-0 truncate block"
                        title={w.name}
                      >
                        {w.name}
                      </button>
                    </td>
                    <td className="py-3 px-3 sm:px-4 align-top min-w-0"><RuntimeBadge runtime={w.runtime} /></td>
                    <td className="py-3 px-3 sm:px-4 align-top min-w-0">
                      <div className="space-y-1 min-w-0">
                        <code
                          className="text-xs bg-zinc-950 px-2 py-1 rounded text-zinc-300 block w-full min-w-0 truncate"
                          title={w.image}
                        >
                          {w.image}
                        </code>
                        {(w.cluster || w.namespace || w.kind) && (
                          <div
                            className="text-[11px] text-zinc-500 truncate"
                            title={[w.kind, w.cluster, w.namespace].filter(Boolean).join(' · ')}
                          >
                            {[w.kind, w.cluster, w.namespace].filter(Boolean).join(' · ')}
                          </div>
                        )}
                      </div>
                    </td>
                    <td className="py-3 px-3 sm:px-4 align-top min-w-0">
                      <Badge text={w.status} variant={getStatusVariant(w.status)} />
                    </td>
                    <td className="py-3 px-3 sm:px-4 text-sm text-zinc-400 align-top min-w-0 whitespace-nowrap">{formatTimestamp(w.created_at)}</td>
                    <td className="py-3 px-3 sm:px-4 align-top min-w-0">
                      <div className="flex flex-wrap gap-1">
                        {isAetherManaged(w) ? (
                          <>
                            <button type="button" onClick={() => openWorkloadDetail(w, 'logs')} className="p-1.5 text-zinc-400 hover:text-blue-400 hover:bg-blue-500/10 rounded transition-colors" title="Logs"><FileText size={14} /></button>
                            <button type="button" onClick={() => handleAction(w.name, 'start')} disabled={actionLoading === `${w.name}-start`} className="p-1.5 text-zinc-400 hover:text-emerald-400 hover:bg-emerald-500/10 rounded transition-colors" title="Start"><Play size={14} /></button>
                            <button type="button" onClick={() => setConfirmAction({ type: 'stop', name: w.name })} disabled={actionLoading === `${w.name}-stop`} className="p-1.5 text-zinc-400 hover:text-amber-400 hover:bg-amber-500/10 rounded transition-colors" title="Stop"><Square size={14} /></button>
                            <button type="button" onClick={() => handleBuild(w.name)} disabled={actionLoading === `${w.name}-build`} className="p-1.5 text-zinc-400 hover:text-teal-400 hover:bg-teal-500/10 rounded transition-colors" title="Build"><Hammer size={14} /></button>
                            <button type="button" onClick={() => setMigrateModal(w.name)} className="p-1.5 text-zinc-400 hover:text-purple-400 hover:bg-purple-500/10 rounded transition-colors" title="Migrate"><ArrowRightLeft size={14} /></button>
                            <button type="button" onClick={() => openWorkloadDetail(w, 'scoring')} className="p-1.5 text-zinc-400 hover:text-cyan-400 hover:bg-cyan-500/10 rounded transition-colors" title="Profile"><Cpu size={14} /></button>
                            <button type="button" onClick={() => openWorkloadDetail(w, 'scoring')} className="p-1.5 text-zinc-400 hover:text-indigo-400 hover:bg-indigo-500/10 rounded transition-colors" title="Analyze"><Search size={14} /></button>
                            <button type="button" onClick={() => openWorkloadDetail(w, 'drift')} className="p-1.5 text-zinc-400 hover:text-orange-400 hover:bg-orange-500/10 rounded transition-colors" title="Drift"><RefreshCw size={14} /></button>
                            <button type="button" onClick={() => setConfirmAction({ type: 'delete', name: w.name })} disabled={actionLoading === `${w.name}-delete`} className="p-1.5 text-zinc-400 hover:text-red-400 hover:bg-red-500/10 rounded transition-colors" title="Delete"><Trash2 size={14} /></button>
                          </>
                        ) : (
                          <span className="text-xs text-zinc-500 px-2 py-1">
                            K8s discovered
                          </span>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {selectedWorkload && (
        <WorkloadDetail
          key={`${selectedWorkload.name}-${detailInitialTab}`}
          workload={selectedWorkload}
          initialTab={detailInitialTab}
          onClose={() => setSelectedWorkload(null)}
          onAction={() => load()}
        />
      )}

      <Modal
        isOpen={confirmAction !== null}
        onClose={() => setConfirmAction(null)}
        title={confirmAction?.type === 'delete' ? 'Confirm Delete' : 'Confirm Stop'}
      >
        <p className="text-sm text-zinc-300 mb-6">
          {confirmAction?.type === 'delete'
            ? `Are you sure you want to delete "${confirmAction.name}"?`
            : `Stop workload "${confirmAction?.name}"?`}
        </p>
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={() => setConfirmAction(null)}
            className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-lg text-sm font-medium transition-colors"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleConfirmedAction}
            className={`px-4 py-2 rounded-lg text-sm font-medium text-white transition-colors ${
              confirmAction?.type === 'delete'
                ? 'bg-red-600 hover:bg-red-500'
                : 'bg-amber-600 hover:bg-amber-500'
            }`}
          >
            {confirmAction?.type === 'delete' ? 'Delete' : 'Stop'}
          </button>
        </div>
      </Modal>

      <Modal isOpen={migrateModal !== null} onClose={() => setMigrateModal(null)} title={`Migrate: ${migrateModal}`}>
        <p className="text-sm text-zinc-400 mb-4">Select target runtime for migration:</p>
        <div className="grid grid-cols-2 gap-3">
          {runtimes.map((rt) => (
            <button
              key={rt}
              type="button"
              onClick={() => migrateModal && handleMigrate(migrateModal, rt)}
              disabled={actionLoading !== null}
              className="px-4 py-3 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-lg text-sm font-medium text-zinc-200 transition-colors capitalize"
            >
              {rt}
            </button>
          ))}
        </div>
      </Modal>

      <Modal isOpen={deployModal} onClose={() => setDeployModal(false)} title="Deploy New Workload">
        <YamlInput
          buttonText="Deploy"
          onSubmit={handleDeploy}
          loading={deployLoading}
          placeholder="Paste workload YAML spec here..."
        />
      </Modal>

      <Modal isOpen={validateModal} onClose={() => { setValidateModal(false); setValidateResult(null); }} title="Validate Workload YAML">
        <YamlInput
          buttonText="Validate"
          onSubmit={handleValidate}
          loading={validateLoading}
          placeholder="Paste workload YAML here..."
        />
        {validateResult && (
          <div className="mt-4">
            <Badge
              text={validateResult.valid ? 'VALID' : 'INVALID'}
              variant={validateResult.valid ? 'green' : 'red'}
            />
            {validateResult.workload_name && (
              <p className="text-sm text-zinc-300 mt-2">Workload: {validateResult.workload_name}</p>
            )}
            {validateResult.errors.length > 0 && (
              <ul className="mt-2 space-y-1">
                {validateResult.errors.map((e, i) => (
                  <li key={i} className="text-sm text-red-400">- {e}</li>
                ))}
              </ul>
            )}
          </div>
        )}
      </Modal>
    </div>
  );
}

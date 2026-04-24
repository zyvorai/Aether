import { useState, useEffect } from 'react';
import { apiFetch, apiPost, apiDelete } from '../utils/api';
import LogViewer from './LogViewer';
import Badge from './Badge';
import type { ClusterResourceDetail, WorkloadResponse } from '../types/api';

interface WorkloadDetailProps {
  workload: WorkloadResponse;
  onClose: () => void;
  onAction: () => void;
}

type DetailTab = 'overview' | 'logs' | 'manifest' | 'drift' | 'scoring';

function getStatusVariant(status: string): 'green' | 'red' | 'yellow' | 'muted' {
  const s = status.toLowerCase();
  if (s === 'running' || s === 'healthy') return 'green';
  if (s === 'error' || s === 'failed') return 'red';
  if (s === 'stopped' || s === 'exited') return 'muted';
  return 'yellow';
}

export default function WorkloadDetail({ workload, onClose, onAction }: WorkloadDetailProps) {
  const isAetherManaged = (workload.source ?? 'aether') === 'aether';
  const isScalableClusterWorkload = !isAetherManaged && ['Deployment', 'StatefulSet'].includes(workload.kind ?? '');
  const clusterResourceName = workload.name.split('/').pop() ?? workload.name;
  const clusterLogsPath = !isAetherManaged && workload.cluster && workload.namespace && workload.kind
    ? `/cluster/logs?cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&kind=${encodeURIComponent(workload.kind)}&name=${encodeURIComponent(clusterResourceName)}`
    : undefined;
  const clusterResourcePath = !isAetherManaged && workload.cluster && workload.namespace && workload.kind
    ? `/cluster/resource?cluster=${encodeURIComponent(workload.cluster)}&namespace=${encodeURIComponent(workload.namespace)}&kind=${encodeURIComponent(workload.kind)}&name=${encodeURIComponent(clusterResourceName)}`
    : undefined;
  const [activeTab, setActiveTab] = useState<DetailTab>('overview');
  const [driftData, setDriftData] = useState<Record<string, unknown> | null>(null);
  const [scoringData, setScoringData] = useState<Record<string, unknown> | null>(null);
  const [clusterDetail, setClusterDetail] = useState<ClusterResourceDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState('');
  const [replicasInput, setReplicasInput] = useState('1');

  useEffect(() => {
    if (activeTab === 'drift' && isAetherManaged) {
      apiFetch<Record<string, unknown>>(`/drift/${workload.name}`).then(r => {
        if (r) setDriftData(r);
      });
    }
  }, [activeTab, isAetherManaged, workload.name]);

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

  const handleAction = async (action: string) => {
    setActionLoading(action);
    try {
      if (isAetherManaged) {
        if (action === 'stop') await apiPost(`/workloads/${workload.name}/stop`);
        else if (action === 'start') await apiPost(`/workloads/${workload.name}/start`);
        else if (action === 'restart') {
          await apiPost(`/workloads/${workload.name}/stop`);
          await apiPost(`/workloads/${workload.name}/start`);
        } else if (action === 'delete') {
          await apiDelete(`/workloads/${workload.name}`);
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
  ];

  return (
    <div className="bg-zinc-900 border border-zinc-700 rounded-xl mt-4 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-zinc-800 border-b border-zinc-700">
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-bold text-white">{workload.name}</h3>
          <Badge text={workload.status} variant={getStatusVariant(workload.status)} />
          <span className="text-sm text-zinc-400">{workload.runtime}</span>
        </div>
        <button onClick={onClose} className="text-zinc-400 hover:text-white text-xl leading-none">&times;</button>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-zinc-700">
        {tabs.map(tab => (
          <button
            key={tab.id}
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
            {isAetherManaged ? (
              <div className="flex gap-2 mb-4">
                {['start', 'stop', 'restart', 'delete'].map(action => (
                  <button
                    key={action}
                    onClick={() => handleAction(action)}
                    disabled={!!actionLoading}
                    className={`px-3 py-1.5 text-sm font-medium rounded transition-colors ${
                      action === 'delete'
                        ? 'bg-red-600/20 text-red-400 hover:bg-red-600/40 border border-red-600/30'
                        : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600 border border-zinc-600'
                    } disabled:opacity-50`}
                  >
                    {actionLoading === action ? '...' : action.charAt(0).toUpperCase() + action.slice(1)}
                  </button>
                ))}
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
          <div className="text-zinc-400 text-sm">
            <p>Click &quot;Analyze&quot; to run the AI scoring engine for this workload.</p>
            <button
              onClick={async () => {
                setLoading(true);
                const r = await apiPost<Record<string, unknown>>('/ai/recommend', {});
                if (r.success && r.data) setScoringData(r.data);
                setLoading(false);
              }}
              className="mt-2 px-3 py-1.5 bg-aether/20 text-aether rounded border border-aether/30 hover:bg-aether/40"
            >
              {loading ? 'Analyzing...' : 'Analyze Runtime'}
            </button>
            {scoringData && (
              <div className="mt-3 bg-zinc-800 rounded p-3">
                <pre className="text-xs text-zinc-300 overflow-x-auto">{JSON.stringify(scoringData, null, 2)}</pre>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

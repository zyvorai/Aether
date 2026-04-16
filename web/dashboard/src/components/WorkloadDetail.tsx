import { useState, useEffect } from 'react';
import { apiFetch, apiPost, apiDelete } from '../utils/api';
import LogViewer from './LogViewer';
import Badge from './Badge';
import type { WorkloadResponse } from '../types/api';

interface WorkloadDetailProps {
  workload: WorkloadResponse;
  onClose: () => void;
  onAction: () => void;
}

type DetailTab = 'overview' | 'logs' | 'drift' | 'scoring';

function getStatusVariant(status: string): 'green' | 'red' | 'yellow' | 'muted' {
  const s = status.toLowerCase();
  if (s === 'running' || s === 'healthy') return 'green';
  if (s === 'error' || s === 'failed') return 'red';
  if (s === 'stopped' || s === 'exited') return 'muted';
  return 'yellow';
}

export default function WorkloadDetail({ workload, onClose, onAction }: WorkloadDetailProps) {
  const [activeTab, setActiveTab] = useState<DetailTab>('overview');
  const [driftData, setDriftData] = useState<Record<string, unknown> | null>(null);
  const [scoringData, setScoringData] = useState<Record<string, unknown> | null>(null);
  const [loading, setLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState('');

  useEffect(() => {
    if (activeTab === 'drift') {
      apiFetch<Record<string, unknown>>(`/drift/${workload.name}`).then(r => {
        if (r) setDriftData(r);
      });
    }
  }, [activeTab, workload.name]);

  const handleAction = async (action: string) => {
    setActionLoading(action);
    try {
      if (action === 'stop') await apiPost(`/workloads/${workload.name}/stop`);
      else if (action === 'start') await apiPost(`/workloads/${workload.name}/start`);
      else if (action === 'restart') {
        await apiPost(`/workloads/${workload.name}/stop`);
        await apiPost(`/workloads/${workload.name}/start`);
      } else if (action === 'delete') {
        await apiDelete(`/workloads/${workload.name}`);
        onClose();
      }
      onAction();
    } catch (e) {
      console.error(`Action ${action} failed:`, e);
    } finally {
      setActionLoading('');
    }
  };

  const tabs: { id: DetailTab; label: string }[] = [
    { id: 'overview', label: 'Overview' },
    { id: 'logs', label: 'Logs' },
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

            {/* Info Grid */}
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div><span className="text-zinc-500">Runtime</span><p className="text-white">{workload.runtime}</p></div>
              <div><span className="text-zinc-500">Image</span><p className="text-white font-mono text-xs">{workload.image}</p></div>
              <div><span className="text-zinc-500">Status</span><p className="text-white">{workload.status}</p></div>
              <div><span className="text-zinc-500">Created</span><p className="text-white">{workload.created_at?.slice(0, 19)}</p></div>
            </div>
          </div>
        )}

        {activeTab === 'logs' && (
          <LogViewer workloadName={workload.name} />
        )}

        {activeTab === 'drift' && (
          <div>
            {driftData ? (
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

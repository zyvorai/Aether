import { useState, useEffect } from 'react';
import { GitBranch, Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import Modal from '../Modal';
import CodeBlock from '../CodeBlock';
import EmptyState from '../EmptyState';
import Badge, { SeverityBadge } from '../Badge';
import type { WorkloadResponse, DriftReport } from '../../types/api';

export default function DriftPage() {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [checkLoading, setCheckLoading] = useState<string | null>(null);
  const [driftResult, setDriftResult] = useState<DriftReport | null>(null);
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    async function load() {
      const data = await apiFetch<WorkloadResponse[]>('/workloads');
      setWorkloads(data ?? []);
      setLoading(false);
    }
    load();
  }, []);

  async function handleCheckDrift(name: string) {
    setCheckLoading(name);
    const data = await apiFetch<DriftReport>(`/drift/${name}`);
    setDriftResult(data);
    setModalOpen(true);
    setCheckLoading(null);
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
      </div>
    );
  }

  return (
    <div>
      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No workloads" description="Deploy a workload to check for drift" />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-2">Select Workload</h2>
          <p className="text-sm text-zinc-400 mb-4">Click a workload to check for configuration drift</p>
          <div className="flex flex-wrap gap-3">
            {workloads.map((w) => (
              <button
                key={w.name}
                onClick={() => handleCheckDrift(w.name)}
                disabled={checkLoading === w.name}
                className="px-4 py-2.5 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-lg text-sm font-medium text-zinc-200 transition-colors disabled:opacity-50"
              >
                {checkLoading === w.name ? 'Checking...' : w.name}
              </button>
            ))}
          </div>
        </div>
      )}

      <Modal
        isOpen={modalOpen}
        onClose={() => { setModalOpen(false); setDriftResult(null); }}
        title={`Drift Report: ${driftResult?.workload_name ?? ''}`}
      >
        {driftResult ? (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <Badge
                text={driftResult.has_drift ? 'DRIFT DETECTED' : 'NO DRIFT'}
                variant={driftResult.has_drift ? 'red' : 'green'}
              />
              <SeverityBadge severity={driftResult.severity} />
            </div>

            {driftResult.drifts.length > 0 && (
              <div>
                <h3 className="text-sm font-medium text-zinc-300 mb-2">Drifted Fields</h3>
                <div className="space-y-2">
                  {driftResult.drifts.map((d, i) => (
                    <div key={i} className="bg-zinc-950/50 rounded-lg p-3">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-sm font-medium text-zinc-200">{d.field}</span>
                        <SeverityBadge severity={d.severity} />
                        <Badge text={d.category} variant="muted" />
                      </div>
                      <div className="grid grid-cols-2 gap-2 text-xs mt-2">
                        <div>
                          <span className="text-zinc-500">Expected: </span>
                          <span className="text-emerald-400">{d.expected}</span>
                        </div>
                        <div>
                          <span className="text-zinc-500">Actual: </span>
                          <span className="text-red-400">{d.actual}</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {driftResult.reconciliation_plan.length > 0 && (
              <div>
                <h3 className="text-sm font-medium text-zinc-300 mb-2">Reconciliation Plan</h3>
                <CodeBlock title="json">{JSON.stringify(driftResult.reconciliation_plan, null, 2)}</CodeBlock>
              </div>
            )}
          </div>
        ) : (
          <EmptyState icon={<GitBranch size={48} />} title="No drift data" />
        )}
      </Modal>
    </div>
  );
}

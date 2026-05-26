// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch, apiFetchSettled, apiPost } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge, { SeverityBadge } from '../Badge';
import type { WorkloadResponse, DriftReport, DriftReconcileResult } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function DriftPage() {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [checkLoading, setCheckLoading] = useState<string | null>(null);
  const [driftResult, setDriftResult] = useState<DriftReport | null>(null);
  const [reconcileLoading, setReconcileLoading] = useState(false);
  const [search, setSearch] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<WorkloadResponse[]>('/workloads');
    if (!result.ok) {
      setLoadFailed(true);
      setWorkloads([]);
    } else {
      setWorkloads(result.data);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleCheckDrift(name: string) {
    setCheckLoading(name);
    const data = await apiFetch<DriftReport>(`/drift/${name}`);
    setDriftResult(data);
    setCheckLoading(null);
  }

  async function handleReconcile(name: string) {
    setReconcileLoading(true);
    const res = await apiPost<{
      reconciled: boolean;
      message?: string;
      results: DriftReconcileResult[];
    }>(`/drift/${name}/reconcile`, {});
    setReconcileLoading(false);
    if (res.success && res.data) {
      if (res.data.reconciled) {
        toast(`Reconciled "${name}": ${res.data.results.length} action(s)`, 'success');
      } else {
        toast(res.data.message ?? 'No drift to reconcile', 'success');
      }
      void handleCheckDrift(name);
    } else {
      toast(res.error ?? 'Reconciliation failed', 'error');
    }
  }

  const filtered = workloads.filter((w) =>
    w.name.toLowerCase().includes(search.toLowerCase())
  );

  if (loading && workloads.length === 0 && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Drift data unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Filter workloads…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No workloads" description="Deploy a workload to check for drift" />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="dash-card">
            <h2 className="text-lg font-semibold text-slate-100 mb-1">Select workload</h2>
            <p className="text-sm text-slate-500 mb-4">Click a workload to check for configuration drift</p>
            <div className="flex flex-wrap gap-2 max-h-[28rem] overflow-auto">
              {filtered.map((w) => (
                <button
                  key={w.name}
                  type="button"
                  onClick={() => void handleCheckDrift(w.name)}
                  disabled={checkLoading === w.name}
                  className={`px-4 py-2 rounded-xl border text-sm font-medium transition-colors disabled:opacity-50 ${
                    driftResult?.workload_name === w.name
                      ? 'border-aether/50 bg-aether/10 text-aether'
                      : 'border-slate-700 bg-slate-950/60 text-slate-200 hover:bg-slate-800/80'
                  }`}
                >
                  {checkLoading === w.name ? 'Checking…' : w.name}
                </button>
              ))}
              {filtered.length === 0 && (
                <p className="text-sm text-slate-500">No workloads match your search.</p>
              )}
            </div>
          </div>

          <div className="dash-card min-h-[12rem]">
            <h3 className="text-sm font-medium text-slate-400 uppercase tracking-wider mb-4">Drift report</h3>
            {!driftResult ? (
              <p className="text-sm text-slate-500">Select a workload to see drift analysis.</p>
            ) : (
              <div className="space-y-4">
                <div className="flex items-center gap-3 flex-wrap">
                  <Badge
                    text={driftResult.has_drift ? 'DRIFT DETECTED' : 'NO DRIFT'}
                    variant={driftResult.has_drift ? 'red' : 'green'}
                  />
                  <SeverityBadge severity={driftResult.severity} />
                  <span className="text-sm text-slate-400">{driftResult.workload_name}</span>
                </div>

                {driftResult.drifts.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-slate-300 mb-2">Drifted fields</h4>
                    <div className="space-y-2 max-h-64 overflow-auto">
                      {driftResult.drifts.map((d, i) => (
                        <div key={i} className="bg-slate-950/50 rounded-lg p-3 border border-slate-800">
                          <div className="flex items-center gap-2 mb-1 flex-wrap">
                            <span className="text-sm font-medium text-slate-200">{d.field}</span>
                            <SeverityBadge severity={d.severity} />
                            <Badge text={d.category} variant="muted" />
                          </div>
                          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs mt-2">
                            <div>
                              <span className="text-slate-500">Expected: </span>
                              <span className="text-emerald-400">{d.expected}</span>
                            </div>
                            <div>
                              <span className="text-slate-500">Actual: </span>
                              <span className="text-red-400">{d.actual}</span>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {driftResult.has_drift && driftResult.reconciliation_plan.length > 0 && (
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      onClick={() => void handleReconcile(driftResult.workload_name)}
                      disabled={reconcileLoading}
                      className="px-4 py-2 bg-aether hover:bg-aether-light disabled:opacity-50 rounded-xl text-sm font-medium text-white"
                    >
                      {reconcileLoading ? 'Reconciling…' : 'Apply reconciliation'}
                    </button>
                  </div>
                )}

                {driftResult.reconciliation_plan.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-slate-300 mb-2">Reconciliation plan</h4>
                    <div className="space-y-2">
                      {driftResult.reconciliation_plan.map((step, i) => (
                        <div key={i} className="rounded-lg border border-slate-800 bg-slate-950/50 p-3 text-sm">
                          <div className="font-medium text-slate-200">{step.action_type}</div>
                          <p className="text-slate-400 mt-1">{step.description}</p>
                          <div className="flex gap-3 mt-2 text-xs text-slate-500">
                            <span>Restart: {step.requires_restart ? 'yes' : 'no'}</span>
                            <span>Risk: {step.risk}</span>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

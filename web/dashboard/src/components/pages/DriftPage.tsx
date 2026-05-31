// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useRef } from 'react';
import { Link, useNavigate } from 'react-router';
import { Inbox } from 'lucide-react';
import { apiFetch, apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam, useWorkloadOrSearchFilter } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import Badge, { SeverityBadge } from '../Badge';
import type { WorkloadResponse, DriftReport, DriftReconcileResult, FleetDriftSummary } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

interface BulkScanState {
  scanning: boolean;
  total: number;
  drifted: string[];
}

export default function DriftPage() {
  const navigate = useNavigate();
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [checkLoading, setCheckLoading] = useState<string | null>(null);
  const [driftResult, setDriftResult] = useState<DriftReport | null>(null);
  const [reconcileLoading, setReconcileLoading] = useState(false);
  const [bulkScan, setBulkScan] = useState<BulkScanState | null>(null);
  const [fleetDrift, setFleetDrift] = useState<FleetDriftSummary | null>(null);
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [workloadParam] = useQueryParam('workload');
  const scannedWorkloadRef = useRef<string | null>(null);
  const workloadFocus = workloadParam.trim() || undefined;

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [result, fleetRes] = await Promise.all([
      apiFetchSettled<WorkloadResponse[]>('/workloads'),
      apiFetchSettled<FleetDriftSummary>('/fleet/drift'),
    ]);
    if (!result.ok) {
      setLoadFailed(true);
      setWorkloads([]);
    } else {
      setWorkloads(result.data);
    }
    setFleetDrift(fleetRes.ok ? fleetRes.data : null);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!workloadFocus || workloads.length === 0) return;
    const match = workloads.find((w) => w.name === workloadFocus);
    if (match && scannedWorkloadRef.current !== workloadFocus) {
      scannedWorkloadRef.current = workloadFocus;
      void handleCheckDrift(match.name);
    }
  }, [workloadFocus, workloads]);

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

  async function handleBulkScan() {
    const targets = filtered;
    if (targets.length === 0) return;
    setBulkScan({ scanning: true, total: targets.length, drifted: [] });
    const drifted: string[] = [];
    for (const w of targets) {
      const data = await apiFetch<DriftReport>(`/drift/${w.name}`);
      if (data?.has_drift) drifted.push(w.name);
    }
    setBulkScan({ scanning: false, total: targets.length, drifted });
    if (drifted.length === 1) {
      void handleCheckDrift(drifted[0]);
    } else if (drifted.length === 0) {
      setDriftResult(null);
    }
  }

  async function handleReconcileAll() {
    if (!bulkScan?.drifted.length) return;
    if (!window.confirm(`Reconcile drift for ${bulkScan.drifted.length} workload(s)?`)) return;
    setReconcileLoading(true);
    let reconciled = 0;
    for (const name of bulkScan.drifted) {
      const res = await apiPost<{ reconciled: boolean }>(`/drift/${name}/reconcile`, {});
      if (res.success && res.data?.reconciled) reconciled++;
    }
    setReconcileLoading(false);
    toast(`Bulk reconcile: ${reconciled} of ${bulkScan.drifted.length} workload(s)`, 'success');
    void handleBulkScan();
  }

  function openWorkloadDrift(name: string) {
    navigate(pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'drift' }));
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
        actions={
          filtered.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                data-testid="drift-bulk-scan"
                onClick={() => void handleBulkScan()}
                disabled={bulkScan?.scanning}
                className="btn-secondary disabled:opacity-50"
              >
                {bulkScan?.scanning ? 'Scanning…' : 'Scan all workloads'}
              </button>
              {bulkScan && !bulkScan.scanning && bulkScan.drifted.length > 0 ? (
                <button
                  type="button"
                  data-testid="drift-reconcile-all-confirm"
                  onClick={() => void handleReconcileAll()}
                  disabled={reconcileLoading}
                  className="btn-primary disabled:opacity-50"
                >
                  {reconcileLoading ? 'Reconciling…' : `Reconcile all (${bulkScan.drifted.length})`}
                </button>
              ) : null}
            </div>
          ) : undefined
        }
      />

      {fleetDrift ? (
        <section className="overview-section-shell mb-6 grid grid-cols-2 sm:grid-cols-4 gap-3 p-6 sm:p-8" data-testid="fleet-drift-summary">
          <div className="dash-card py-3 px-4"><div className="text-xs text-slate-500">Tracked</div><div className="text-lg font-semibold text-slate-100">{fleetDrift.total_workloads}</div></div>
          <div className="dash-card py-3 px-4"><div className="text-xs text-slate-500">Drifted</div><div className="text-lg font-semibold text-amber-300">{fleetDrift.drifted}</div></div>
          <div className="dash-card py-3 px-4"><div className="text-xs text-slate-500">Critical</div><div className="text-lg font-semibold text-red-400">{fleetDrift.critical}</div></div>
          <div className="dash-card py-3 px-4"><div className="text-xs text-slate-500">Warnings</div><div className="text-lg font-semibold text-yellow-300">{fleetDrift.warning}</div></div>
        </section>
      ) : null}

      <div className="mb-4">
        <button
          type="button"
          data-testid="drift-events-link"
          onClick={() =>
            navigate(
              pathWithQuery(viewToPath('events'), {
                category: 'drift',
                ...(workloadFocus ? { workload: workloadFocus } : {}),
              }),
            )
          }
          className="text-xs text-aether hover:underline"
        >
          Drift events →
        </button>
      </div>

      {workloadFocus ? (
        <WorkloadContextBanner
          testId="drift-workload-context"
          workload={workloadFocus}
          openTestId="drift-open-workload"
          description="Drift context"
        >
          <WorkloadScopedCrossLinks workload={workloadFocus} prefix="drift" eventsCategory="drift" showGitops showAudit showMetrics />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="drift-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="drift-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="drift-context-compose-link"
          >
            Compose →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="drift-context-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="drift-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="drift-context-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('copilot'), { workload: workloadFocus, q: `Explain drift for ${workloadFocus}` })}
            className="text-aether hover:underline"
            data-testid="drift-context-copilot-link"
          >
            Copilot →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      {bulkScan && !bulkScan.scanning ? (
        <div data-testid="drift-bulk-summary" className="dash-card mb-6 text-sm text-zinc-300">
          Scanned {bulkScan.total} workload(s) — {bulkScan.drifted.length} with drift
          {bulkScan.drifted.length > 0 ? (
            <span className="ml-2 text-zinc-500">({bulkScan.drifted.join(', ')})</span>
          ) : null}
          <button
            type="button"
            onClick={() =>
              navigate(
                workloadFocus
                  ? pathWithQuery(viewToPath('gitops'), { workload: workloadFocus })
                  : viewToPath('gitops'),
              )
            }
            className="ml-3 text-xs text-aether hover:underline"
            data-testid="drift-bulk-gitops-link"
          >
            GitOps sync →
          </button>
        </div>
      ) : null}

      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No workloads" description="Deploy a workload to check for drift" />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="dash-card">
            <h2 className="text-lg font-semibold text-zinc-100 mb-1">Select workload</h2>
            <p className="text-sm text-zinc-500 mb-4">Click a workload to check for configuration drift</p>
            <div className="flex flex-wrap gap-2 max-h-[28rem] overflow-auto" data-testid="drift-workload-select">
              {filtered.map((w) => (
                <button
                  key={w.name}
                  type="button"
                  onClick={() => void handleCheckDrift(w.name)}
                  disabled={checkLoading === w.name}
                  data-testid={workloadFocus === w.name ? 'drift-workload-highlight' : undefined}
                  className={`px-4 py-2 rounded-xl border text-sm font-medium transition-colors disabled:opacity-50 ${
                    driftResult?.workload_name === w.name
                      ? 'border-aether/50 bg-aether/10 text-aether'
                      : workloadFocus === w.name
                        ? 'border-aether/60 ring-1 ring-aether/30 bg-aether/5 text-aether'
                        : bulkScan?.drifted.includes(w.name)
                          ? 'border-red-500/40 bg-red-500/10 text-red-300'
                          : 'border-zinc-700 bg-zinc-950/60 text-zinc-200 hover:bg-zinc-800/80'
                  }`}
                >
                  {checkLoading === w.name ? 'Checking…' : w.name}
                </button>
              ))}
              {filtered.length === 0 && (
                <p className="text-sm text-zinc-500">No workloads match your search.</p>
              )}
            </div>
          </div>

          <div className="dash-card min-h-[12rem]">
            <h3 className="text-sm font-medium text-zinc-400 uppercase tracking-wider mb-4">Drift report</h3>
            {!driftResult ? (
              <p className="text-sm text-zinc-500">Select a workload or run a bulk scan to see drift analysis.</p>
            ) : (
              <div className="space-y-4" data-testid="drift-result-panel">
                <div className="flex items-center gap-3 flex-wrap">
                  <Badge
                    text={driftResult.has_drift ? 'DRIFT DETECTED' : 'NO DRIFT'}
                    variant={driftResult.has_drift ? 'red' : 'green'}
                  />
                  <SeverityBadge severity={driftResult.severity} />
                  <button
                    type="button"
                    data-testid="drift-open-workload"
                    onClick={() => openWorkloadDrift(driftResult.workload_name)}
                    className="text-sm text-aether hover:underline"
                  >
                    {driftResult.workload_name} → detail
                  </button>
                </div>

                {driftResult.drifts.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-zinc-300 mb-2">Drifted fields</h4>
                    <div className="space-y-2 max-h-64 overflow-auto">
                      {driftResult.drifts.map((d, i) => (
                        <div key={i} className="bg-zinc-950/50 rounded-lg p-3 border border-zinc-800">
                          <div className="flex items-center gap-2 mb-1 flex-wrap">
                            <span className="text-sm font-medium text-zinc-200">{d.field}</span>
                            <SeverityBadge severity={d.severity} />
                            <Badge text={d.category} variant="muted" />
                          </div>
                          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs mt-2">
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

                {driftResult.has_drift && driftResult.reconciliation_plan.length > 0 && (
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      data-testid="drift-reconcile-button"
                      onClick={() => void handleReconcile(driftResult.workload_name)}
                      disabled={reconcileLoading}
                      className="btn-primary disabled:opacity-50"
                    >
                      {reconcileLoading ? 'Reconciling…' : 'Apply reconciliation'}
                    </button>
                  </div>
                )}

                {driftResult.reconciliation_plan.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-zinc-300 mb-2">Reconciliation plan</h4>
                    <div className="space-y-2">
                      {driftResult.reconciliation_plan.map((step, i) => (
                        <div key={i} className="rounded-lg border border-zinc-800 bg-zinc-950/50 p-3 text-sm">
                          <div className="font-medium text-zinc-200">{step.action_type}</div>
                          <p className="text-zinc-400 mt-1">{step.description}</p>
                          <div className="flex gap-3 mt-2 text-xs text-zinc-500">
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

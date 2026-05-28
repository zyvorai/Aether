// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router';
import { Inbox } from 'lucide-react';
import { apiFetch, apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam, useWorkloadOrSearchFilter } from '../../utils/urlState';
import { useAuth } from '../../contexts/AuthContext';
import { markHealthReviewed } from '../../utils/onboardingState';
import PageToolbar from '../PageToolbar';
import StatCard from '../StatCard';
import Badge, { RuntimeBadge } from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { HealthSummary, ManagedWorkload, HealthHistorySummary } from '../../types/api';

function getHealthVariant(health: string): 'green' | 'yellow' | 'red' | 'muted' {
  const h = health.toLowerCase();
  if (h === 'healthy') return 'green';
  if (h === 'degraded') return 'yellow';
  if (h === 'unhealthy') return 'red';
  return 'muted';
}

function getCircuitVariant(circuit: string): 'green' | 'red' | 'yellow' | 'muted' {
  const c = circuit.toLowerCase();
  if (c === 'closed') return 'green';
  if (c === 'open') return 'red';
  if (c === 'half-open' || c === 'half_open') return 'yellow';
  return 'muted';
}

export default function HealthPage() {
  const navigate = useNavigate();
  const [summary, setSummary] = useState<HealthSummary | null>(null);
  const [workloads, setWorkloads] = useState<ManagedWorkload[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [statusFilter, setStatusFilter] = useQueryParam('status', 'all');
  const [workloadParam, setWorkloadParam] = useQueryParam('workload');
  const [selected, setSelected] = useState<{ workload: ManagedWorkload; history: HealthHistorySummary } | null>(null);
  const [historyLoading, setHistoryLoading] = useState<string | null>(null);
  const [orchBusy, setOrchBusy] = useState<string | null>(null);
  const [rollingReplicas, setRollingReplicas] = useState('2');
  const [rollingMsg, setRollingMsg] = useState<string | null>(null);
  const { canMutate } = useAuth();

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [s, w] = await Promise.all([
      apiFetchSettled<HealthSummary>('/orchestrator/summary'),
      apiFetchSettled<ManagedWorkload[]>('/orchestrator/status'),
    ]);
    if (!s.ok && !w.ok) {
      setLoadFailed(true);
      setSummary(null);
      setWorkloads([]);
    } else {
      setSummary(s.ok ? s.data : null);
      setWorkloads(w.ok ? w.data : []);
    }
    setLoading(false);
    if (s.ok || w.ok) {
      markHealthReviewed();
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!workloadParam || workloads.length === 0) return;
    const match = workloads.find((w) => w.name === workloadParam);
    if (match && selected?.workload.name !== match.name) {
      void handleRowClick(match);
    }
  }, [workloadParam, workloads, selected?.workload.name]);

  async function handleRowClick(w: ManagedWorkload) {
    if (selected?.workload.name === w.name) {
      setSelected(null);
      return;
    }
    setHistoryLoading(w.name);
    const data = await apiFetch<HealthHistorySummary>(`/health/${w.name}`);
    setHistoryLoading(null);
    if (data) {
      setSelected({ workload: w, history: data });
    }
  }

  async function runOrchestratorAction(action: 'health-check' | 'reset-circuit', name?: string) {
    if (!canMutate) return;
    setOrchBusy(action);
    if (action === 'health-check') {
      await apiPost('/orchestrator/health-check', {});
    } else if (name) {
      await apiPost('/orchestrator/reset-circuit', { name });
    }
    setOrchBusy(null);
    void load();
  }

  async function runRollingUpdate(name: string) {
    if (!canMutate) return;
    const replicas = parseInt(rollingReplicas, 10);
    if (!Number.isFinite(replicas) || replicas < 1) {
      setRollingMsg('Replicas must be a positive integer');
      return;
    }
    setOrchBusy('rolling-update');
    setRollingMsg(null);
    const res = await apiPost<{ message?: string }>('/orchestrator/rolling-update', { name, replicas });
    setOrchBusy(null);
    if (res.success) {
      setRollingMsg(`Rolling update started for ${name} (${replicas} replicas)`);
      void load();
    } else {
      setRollingMsg(res.error ?? 'Rolling update failed');
    }
  }

  const filtered = workloads.filter((w) => {
    const matchesSearch = w.name.toLowerCase().includes(search.toLowerCase());
    const status = statusFilter.toLowerCase();
    const matchesStatus = status === 'all' || (w.health ?? '').toLowerCase() === status;
    return matchesSearch && matchesStatus;
  });

  if (loading && workloads.length === 0 && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Health data unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <WorkloadContextBanner
        testId="health-workload-context"
        workload={workloadParam}
        description="Health monitor for workload"
      >
        <WorkloadScopedCrossLinks
          workload={workloadParam}
          prefix="health"
          showDrift
          showGitops
          showMetrics
        />
        {workloadParam.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('platform'), { workload: workloadParam.trim() })}
              className="text-aether hover:underline"
              data-testid="health-platform-link"
            >
              Platform →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: workloadParam.trim() })}
              className="text-aether hover:underline"
              data-testid="health-openapi-link"
            >
              OpenAPI →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('copilot'), { workload: workloadParam.trim(), q: `Why is ${workloadParam.trim()} unhealthy?` })}
              className="text-aether hover:underline"
              data-testid="health-copilot-link"
            >
              Copilot →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: workloadParam.trim() })}
              className="text-aether hover:underline"
              data-testid="health-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: workloadParam.trim() })}
              className="text-aether hover:underline"
              data-testid="health-context-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('intelligence'), { workload: workloadParam.trim(), tab: 'predictions' })}
              className="text-aether hover:underline"
              data-testid="health-context-intelligence-link"
            >
              Intelligence →
            </Link>
          </>
        ) : null}
      </WorkloadContextBanner>
      {summary && (
        <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
          <button type="button" data-testid="health-healthy-stat" onClick={() => setStatusFilter('healthy')} className="text-left">
            <StatCard title="Healthy" value={summary.healthy} color="green" />
          </button>
          <button type="button" data-testid="health-degraded-stat" onClick={() => setStatusFilter('degraded')} className="text-left">
            <StatCard title="Degraded" value={summary.degraded} color="yellow" />
          </button>
          <button type="button" data-testid="health-unhealthy-stat" onClick={() => setStatusFilter('unhealthy')} className="text-left">
            <StatCard title="Unhealthy" value={summary.unhealthy} color="red" />
          </button>
          <button type="button" data-testid="health-unknown-stat" onClick={() => setStatusFilter('unknown')} className="text-left">
            <StatCard title="Unknown" value={summary.unknown} color="blue" />
          </button>
          <StatCard title="Circuits open" value={summary.circuits_open} color="orange" />
        </div>
      )}

      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Filter workloads…"
        onRefresh={() => void load()}
        refreshing={loading || orchBusy !== null}
        actions={
          canMutate ? (
            <button
              type="button"
              data-testid="health-run-checks"
              onClick={() => void runOrchestratorAction('health-check')}
              disabled={orchBusy !== null}
              className="rounded-xl bg-emerald-600/20 border border-emerald-500/40 px-4 py-2 text-sm text-emerald-300 hover:bg-emerald-600/30 disabled:opacity-50"
            >
              {orchBusy === 'health-check' ? 'Checking…' : 'Run health checks'}
            </button>
          ) : null
        }
        filters={
          <>
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="rounded-lg border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm text-slate-100"
            aria-label="Health status filter"
            data-testid="health-status-filter"
          >
            <option value="all">All statuses</option>
            <option value="healthy">Healthy</option>
            <option value="degraded">Degraded</option>
            <option value="unhealthy">Unhealthy</option>
            <option value="unknown">Unknown</option>
          </select>
          {statusFilter !== 'all' && (
            <button
              type="button"
              data-testid="health-clear-filter"
              onClick={() => setStatusFilter('all')}
              className="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-400 hover:text-aether"
            >
              Clear filter
            </button>
          )}
          </>
        }
      />

      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No managed workloads" description="No workloads are being monitored" />
      ) : (
        <>
          <div className="dash-card overflow-hidden mb-6" data-testid="health-workload-table">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-slate-800">
                    <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Workload</th>
                    <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Runtime</th>
                    <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Health</th>
                    <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Circuit</th>
                    <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Restarts</th>
                  </tr>
                </thead>
                <tbody>
                  {filtered.map((w) => (
                    <tr
                      key={w.name}
                      onClick={() => void handleRowClick(w)}
                      className={`border-b border-slate-800/50 cursor-pointer transition-colors ${
                        selected?.workload.name === w.name ? 'bg-aether/10' : 'hover:bg-slate-800/30'
                      }`}
                    >
                      <td className="py-3 px-4 font-medium text-slate-200">
                        {historyLoading === w.name ? (
                          <span className="text-slate-500">Loading…</span>
                        ) : (
                          w.name
                        )}
                      </td>
                      <td className="py-3 px-4">
                        <RuntimeBadge runtime={w.runtime} />
                      </td>
                      <td className="py-3 px-4">
                        <Badge text={w.health} variant={getHealthVariant(w.health)} />
                      </td>
                      <td className="py-3 px-4">
                        <Badge text={w.circuit ?? 'unknown'} variant={getCircuitVariant(w.circuit ?? 'unknown')} />
                      </td>
                      <td className="py-3 px-4 text-sm text-slate-300">{w.restart_count}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {filtered.length === 0 && (
                <p className="text-sm text-slate-500 py-6 text-center">No workloads match your search.</p>
              )}
            </div>
          </div>

          {selected && (
            <div className="dash-card" data-testid="health-detail-panel">
              <h3 className="text-lg font-semibold text-slate-100 mb-4">
                Health detail:{' '}
                <button
                  type="button"
                  onClick={() =>
                    navigate(pathWithQuery(viewToPath('workloads'), { workload: selected.workload.name }))
                  }
                  className="text-aether hover:underline"
                >
                  {selected.workload.name}
                </button>
              </h3>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                <div className="rounded-xl border border-slate-800 bg-slate-950/60 p-3">
                  <div className="text-xs text-slate-500 mb-1">Total checks</div>
                  <div className="text-lg font-semibold text-slate-100">{selected.history.total_checks}</div>
                </div>
                <div className="rounded-xl border border-slate-800 bg-slate-950/60 p-3">
                  <div className="text-xs text-slate-500 mb-1">Ready checks</div>
                  <div className="text-lg font-semibold text-emerald-400">{selected.history.ready_checks}</div>
                </div>
                <div className="rounded-xl border border-slate-800 bg-slate-950/60 p-3">
                  <div className="text-xs text-slate-500 mb-1">Uptime</div>
                  <div className="text-lg font-semibold text-slate-100">{selected.history.uptime_percent.toFixed(2)}%</div>
                </div>
                <div className="rounded-xl border border-slate-800 bg-slate-950/60 p-3">
                  <div className="text-xs text-slate-500 mb-1">Last state</div>
                  <div className="text-lg font-semibold text-slate-100">{selected.history.last_state}</div>
                </div>
              </div>
              <div className="flex flex-wrap gap-4 text-sm text-slate-400">
                <span>
                  Runtime: <span className="text-slate-200">{selected.workload.runtime}</span>
                </span>
                <span>
                  Circuit: <Badge text={selected.workload.circuit} variant={getCircuitVariant(selected.workload.circuit)} />
                </span>
                {canMutate && selected.workload.circuit.toLowerCase() !== 'closed' && (
                  <button
                    type="button"
                    onClick={() => void runOrchestratorAction('reset-circuit', selected.workload.name)}
                    className="text-xs rounded-lg border border-amber-500/40 px-2 py-1 text-amber-300 hover:bg-amber-500/10"
                  >
                    Reset circuit
                  </button>
                )}
                {canMutate && (
                  <div className="flex flex-wrap items-center gap-2" data-testid="health-rolling-update">
                    <label className="text-xs text-slate-500" htmlFor="rolling-replicas">
                      Rolling update replicas
                    </label>
                    <input
                      id="rolling-replicas"
                      type="number"
                      min={1}
                      value={rollingReplicas}
                      onChange={(e) => setRollingReplicas(e.target.value)}
                      className="w-16 rounded-lg border border-slate-700 bg-slate-950/80 px-2 py-1 text-xs text-slate-100"
                    />
                    <button
                      type="button"
                      data-testid="health-rolling-update-submit"
                      onClick={() => void runRollingUpdate(selected.workload.name)}
                      disabled={orchBusy !== null}
                      className="text-xs rounded-lg border border-emerald-500/40 px-2 py-1 text-emerald-300 hover:bg-emerald-500/10 disabled:opacity-50"
                    >
                      {orchBusy === 'rolling-update' ? 'Updating…' : 'Rolling update'}
                    </button>
                  </div>
                )}
                {rollingMsg ? <span className="text-xs text-slate-400">{rollingMsg}</span> : null}
                <Link
                  to={pathWithQuery(viewToPath('events'), { workload: selected.workload.name })}
                  className="text-xs text-aether hover:underline"
                >
                  View events →
                </Link>
                <Link
                  to={pathWithQuery(viewToPath('gitops'), { workload: selected.workload.name })}
                  className="text-xs text-aether hover:underline"
                  data-testid="health-gitops-link"
                >
                  GitOps →
                </Link>
                <Link
                  to={pathWithQuery(viewToPath('drift'), { workload: selected.workload.name })}
                  className="text-xs text-aether hover:underline"
                  data-testid="health-drift-link"
                >
                  Drift →
                </Link>
                <Link
                  to={pathWithQuery(viewToPath('alerts'), { workload: selected.workload.name })}
                  className="text-xs text-aether hover:underline"
                  data-testid="health-alerts-link"
                >
                  Alert rules →
                </Link>
                <Link
                  to={pathWithQuery(viewToPath('workloads'), {
                    workload: selected.workload.name,
                    tab: 'trust',
                  })}
                  className="text-xs text-aether hover:underline"
                  data-testid="health-trust-link"
                >
                  Trust & attestation →
                </Link>
                <Link
                  to={viewToPath('sla')}
                  className="text-xs text-aether hover:underline"
                  data-testid="health-sla-link"
                >
                  SLA compliance →
                </Link>
                <span>
                  Last restart count: <span className="text-slate-200">{selected.history.last_restart_count}</span>
                </span>
                <span>
                  Current restarts: <span className="text-slate-200">{selected.workload.restart_count}</span>
                </span>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

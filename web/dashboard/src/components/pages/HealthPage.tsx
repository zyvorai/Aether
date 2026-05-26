// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch, apiFetchSettled } from '../../utils/api';
import { useQueryParam } from '../../utils/urlState';
import { markHealthReviewed } from '../../utils/onboardingState';
import PageToolbar from '../PageToolbar';
import StatCard from '../StatCard';
import Badge, { RuntimeBadge } from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
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
  const [summary, setSummary] = useState<HealthSummary | null>(null);
  const [workloads, setWorkloads] = useState<ManagedWorkload[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useQueryParam('status', 'all');
  const [selected, setSelected] = useState<{ workload: ManagedWorkload; history: HealthHistorySummary } | null>(null);
  const [historyLoading, setHistoryLoading] = useState<string | null>(null);

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

  const filtered = workloads.filter((w) => {
    const matchesSearch = w.name.toLowerCase().includes(search.toLowerCase());
    const status = statusFilter.toLowerCase();
    const matchesStatus = status === 'all' || w.health.toLowerCase() === status;
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
      {summary && (
        <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
          <StatCard title="Healthy" value={summary.healthy} color="green" />
          <StatCard title="Degraded" value={summary.degraded} color="yellow" />
          <StatCard title="Unhealthy" value={summary.unhealthy} color="red" />
          <StatCard title="Unknown" value={summary.unknown} color="blue" />
          <StatCard title="Circuits open" value={summary.circuits_open} color="orange" />
        </div>
      )}

      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Filter workloads…"
        onRefresh={() => void load()}
        refreshing={loading}
        filters={
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="rounded-lg border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm text-slate-100"
            aria-label="Health status filter"
          >
            <option value="all">All statuses</option>
            <option value="healthy">Healthy</option>
            <option value="degraded">Degraded</option>
            <option value="unhealthy">Unhealthy</option>
            <option value="unknown">Unknown</option>
          </select>
        }
      />

      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No managed workloads" description="No workloads are being monitored" />
      ) : (
        <>
          <div className="dash-card overflow-hidden mb-6">
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
                        <Badge text={w.circuit} variant={getCircuitVariant(w.circuit)} />
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
            <div className="dash-card">
              <h3 className="text-lg font-semibold text-slate-100 mb-4">
                Health detail: {selected.workload.name}
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

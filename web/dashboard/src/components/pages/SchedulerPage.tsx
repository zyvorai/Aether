// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatUSD } from '../../utils/formatters';
import StatCard from '../StatCard';
import BarChart from '../BarChart';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { RuntimeUtilization, OptimizeSuggestion } from '../../types/api';

export default function SchedulerPage() {
  const [utilization, setUtilization] = useState<RuntimeUtilization[]>([]);
  const [suggestions, setSuggestions] = useState<OptimizeSuggestion[]>([]);
  const [placements, setPlacements] = useState<Array<{ workload_name: string; runtime: string; cpu_reserved: number; memory_reserved_mb: number; placed_at: string }>>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  const load = useCallback(async () => {
    setLoadFailed(false);
    const [u, s, p] = await Promise.all([
      apiFetchSettled<RuntimeUtilization[]>('/scheduler/utilization'),
      apiFetchSettled<OptimizeSuggestion[]>('/scheduler/optimize'),
      apiFetchSettled<Array<{ workload_name: string; runtime: string; cpu_reserved: number; memory_reserved_mb: number; placed_at: string }>>('/scheduler/placements'),
    ]);
    if (!u.ok && !s.ok && !p.ok) {
      setLoadFailed(true);
      setUtilization([]);
      setSuggestions([]);
      setPlacements([]);
    } else {
      setUtilization(u.ok ? u.data : []);
      setSuggestions(s.ok ? s.data : []);
      setPlacements(p.ok ? p.data : []);
    }
  }, []);

  useEffect(() => {
    void (async () => {
      await load();
      setLoading(false);
    })();
  }, [load]);

  async function handleRefresh() {
    setRefreshing(true);
    await load();
    setRefreshing(false);
  }

  if (loading && !loadFailed) {
    return <PageLoading rows={6} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Scheduler data unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <PageToolbar onRefresh={() => void handleRefresh()} refreshing={refreshing} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        {utilization.length === 0 ? (
          <EmptyState icon={<Inbox size={48} />} title="No utilization data" description="No runtimes are reporting utilization" />
        ) : (
          utilization.map((rt) => (
            <div key={rt.runtime} className="dash-card">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-zinc-100 capitalize">{rt.runtime}</h2>
                <div className="flex items-center gap-2">
                  <Badge
                    text={rt.healthy ? 'Healthy' : 'Unhealthy'}
                    variant={rt.healthy ? 'green' : 'red'}
                  />
                  <span className="text-xs text-zinc-500">{formatUSD(rt.estimated_cost_per_day)}/day</span>
                </div>
              </div>

              <div className="mb-4">
                <StatCard title="Workloads" value={`${rt.workload_count}/${rt.max_workloads}`} color="blue" />
              </div>

              <div className="space-y-3">
                <BarChart label="CPU Utilization" percent={rt.cpu_utilization * 100} />
                <BarChart label="Memory Utilization" percent={rt.memory_utilization * 100} />
              </div>
            </div>
          ))
        )}
      </div>

      <div className="dash-card mb-6">
        <h2 className="text-lg font-semibold text-zinc-100 mb-4">Current placements</h2>
        {placements.length === 0 ? (
          <EmptyState icon={<Inbox size={48} />} title="No placements" description="No scheduler placement records yet" />
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-slate-800 text-xs uppercase text-slate-500">
                  <th className="py-2 px-3 text-left">Workload</th>
                  <th className="py-2 px-3 text-left">Runtime</th>
                  <th className="py-2 px-3 text-left">CPU</th>
                  <th className="py-2 px-3 text-left">Memory</th>
                </tr>
              </thead>
              <tbody>
                {placements.map((p) => (
                  <tr key={p.workload_name} className="border-b border-slate-800/50">
                    <td className="py-2 px-3">{p.workload_name}</td>
                    <td className="py-2 px-3">{p.runtime}</td>
                    <td className="py-2 px-3">{p.cpu_reserved}</td>
                    <td className="py-2 px-3">{p.memory_reserved_mb} MB</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="dash-card">
        <h2 className="text-lg font-semibold text-zinc-100 mb-4">Optimization Suggestions</h2>
        {suggestions.length === 0 ? (
          <EmptyState icon={<Inbox size={48} />} title="No suggestions" description="No optimization suggestions at this time" />
        ) : (
          <div className="space-y-3">
            {suggestions.map((s, i) => (
              <div key={i} className="flex items-start gap-3 p-3 bg-zinc-950/50 rounded-lg">
                <Badge text={s.category} variant="blue" />
                <div className="flex-1">
                  <div className="text-sm text-zinc-200">{s.message}</div>
                  {s.potential_saving !== null && (
                    <div className="text-xs text-emerald-400 mt-1">
                      Potential saving: {formatUSD(s.potential_saving)}/day
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

import { useState, useEffect } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { formatUSD } from '../../utils/formatters';
import StatCard from '../StatCard';
import BarChart from '../BarChart';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import type { RuntimeUtilization, OptimizeSuggestion } from '../../types/api';

export default function SchedulerPage() {
  const [utilization, setUtilization] = useState<RuntimeUtilization[]>([]);
  const [suggestions, setSuggestions] = useState<OptimizeSuggestion[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      const [u, s] = await Promise.all([
        apiFetch<RuntimeUtilization[]>('/scheduler/utilization'),
        apiFetch<OptimizeSuggestion[]>('/scheduler/optimize'),
      ]);
      setUtilization(u ?? []);
      setSuggestions(s ?? []);
      setLoading(false);
    }
    load();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
      </div>
    );
  }

  return (
    <div>
      {/* Runtime Utilization */}
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

      {/* Optimization Suggestions */}
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

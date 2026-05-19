import { useState, useEffect, useCallback } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import EmptyState from '../EmptyState';
import type { WorkloadResponse, SlaTarget } from '../../types/api';

export default function SLAPage() {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [slaData, setSlaData] = useState<Record<string, SlaTarget | null>>({});
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<WorkloadResponse[]>('/workloads');
    const wl = data ?? [];
    setWorkloads(wl);
    const results: Record<string, SlaTarget | null> = {};
    await Promise.all(
      wl.map(async (w) => {
        const sla = await apiFetch<SlaTarget>(`/sla/${w.name}`);
        results[w.name] = sla;
      })
    );
    setSlaData(results);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = workloads.filter((w) => w.name.toLowerCase().includes(search.toLowerCase()));

  if (loading && workloads.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-aether" />
      </div>
    );
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
        <EmptyState icon={<Inbox size={48} />} title="No workloads" description="Deploy a workload to configure SLA targets" />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No matches" description="Try a different search term" />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {filtered.map((w) => {
            const sla = slaData[w.name];
            return (
              <div key={w.name} className="dash-card">
                <h2 className="text-lg font-semibold text-slate-100 mb-4">{w.name}</h2>
                {sla ? (
                  <dl className="space-y-3 text-sm">
                    <div className="flex justify-between">
                      <dt className="text-slate-400">Uptime target</dt>
                      <dd className="text-emerald-400 font-medium">{sla.uptime_target_pct}%</dd>
                    </div>
                    {sla.max_latency_ms !== null && (
                      <div className="flex justify-between">
                        <dt className="text-slate-400">Max latency</dt>
                        <dd className="text-slate-200">{sla.max_latency_ms}ms</dd>
                      </div>
                    )}
                    {sla.max_error_rate_pct !== null && (
                      <div className="flex justify-between">
                        <dt className="text-slate-400">Max error rate</dt>
                        <dd className="text-slate-200">{sla.max_error_rate_pct}%</dd>
                      </div>
                    )}
                    {sla.max_restarts_per_day !== null && (
                      <div className="flex justify-between">
                        <dt className="text-slate-400">Max restarts/day</dt>
                        <dd className="text-slate-200">{sla.max_restarts_per_day}</dd>
                      </div>
                    )}
                  </dl>
                ) : (
                  <p className="text-sm text-slate-500">No SLA configured</p>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

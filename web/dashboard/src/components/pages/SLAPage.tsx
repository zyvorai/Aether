import { useState, useEffect } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import CodeBlock from '../CodeBlock';
import EmptyState from '../EmptyState';
import type { WorkloadResponse, SlaTarget } from '../../types/api';

export default function SLAPage() {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [slaData, setSlaData] = useState<Record<string, SlaTarget | null>>({});
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
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
      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No workloads" description="Deploy a workload to configure SLA targets" />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {workloads.map((w) => {
            const sla = slaData[w.name];
            return (
              <div key={w.name} className="dash-card">
                <h2 className="text-lg font-semibold text-zinc-100 mb-4">{w.name}</h2>
                {sla ? (
                  <div className="space-y-3">
                    <div className="flex justify-between text-sm">
                      <span className="text-zinc-400">Uptime Target</span>
                      <span className="text-emerald-400 font-medium">{sla.uptime_target_pct}%</span>
                    </div>
                    {sla.max_latency_ms !== null && (
                      <div className="flex justify-between text-sm">
                        <span className="text-zinc-400">Max Latency</span>
                        <span className="text-zinc-200">{sla.max_latency_ms}ms</span>
                      </div>
                    )}
                    {sla.max_error_rate_pct !== null && (
                      <div className="flex justify-between text-sm">
                        <span className="text-zinc-400">Max Error Rate</span>
                        <span className="text-zinc-200">{sla.max_error_rate_pct}%</span>
                      </div>
                    )}
                    {sla.max_restarts_per_day !== null && (
                      <div className="flex justify-between text-sm">
                        <span className="text-zinc-400">Max Restarts/Day</span>
                        <span className="text-zinc-200">{sla.max_restarts_per_day}</span>
                      </div>
                    )}
                    <CodeBlock title="json">{JSON.stringify(sla, null, 2)}</CodeBlock>
                  </div>
                ) : (
                  <p className="text-sm text-zinc-500">No SLA configured</p>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

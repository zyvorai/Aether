import { useState, useEffect } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import StatCard from '../StatCard';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import type { AuditResponse } from '../../types/api';

export default function AuditPage() {
  const [audit, setAudit] = useState<AuditResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      const data = await apiFetch<AuditResponse>('/audit');
      setAudit(data);
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

  if (!audit) {
    return (
      <EmptyState icon={<Inbox size={48} />} title="No audit data" description="No audit events have been recorded" />
    );
  }

  return (
    <div>
      <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
        <StatCard title="Total Events" value={audit.summary.total_events} color="blue" />
        <StatCard title="Successes" value={audit.summary.successes} color="green" />
        <StatCard title="Failures" value={audit.summary.failures} color="red" />
        <StatCard title="Workloads" value={audit.summary.unique_workloads} color="purple" />
      </div>

      {audit.recent_events.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No recent events" />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Recent Events</h2>
          <div className="space-y-3 max-h-[600px] overflow-auto">
            {audit.recent_events.map((ev) => (
              <div key={ev.id} className="flex items-start gap-3 p-4 bg-zinc-950/50 rounded-lg">
                <Badge
                  text={ev.result}
                  variant={ev.result.toLowerCase() === 'success' ? 'green' : 'red'}
                />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-zinc-200">{ev.action}</span>
                    <span className="text-xs text-zinc-500">on</span>
                    <span className="text-sm text-amber-400">{ev.workload}</span>
                  </div>
                  <div className="text-xs text-zinc-400 mt-1">{ev.message}</div>
                  <div className="flex items-center gap-3 mt-2 text-xs text-zinc-500">
                    <span>{formatTimestamp(ev.timestamp)}</span>
                    {ev.runtime && <span>Runtime: {ev.runtime}</span>}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

import { useState, useEffect } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import StatCard from '../StatCard';
import Badge, { SeverityBadge } from '../Badge';
import EmptyState from '../EmptyState';
import type { Event, EventSummary } from '../../types/api';

export default function EventsPage() {
  const [events, setEvents] = useState<Event[]>([]);
  const [summary, setSummary] = useState<EventSummary | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      const [ev, s] = await Promise.all([
        apiFetch<Event[]>('/events'),
        apiFetch<EventSummary>('/events/summary'),
      ]);
      setEvents(ev ?? []);
      setSummary(s);
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
      {summary && (
        <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
          <StatCard title="Total" value={summary.total_events} color="blue" />
          <StatCard title="Unacknowledged" value={summary.unacknowledged} color="yellow" />
          <StatCard title="Critical" value={summary.critical_unacked} color="red" />
        </div>
      )}

      {events.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No events" description="No events have been recorded" />
      ) : (
        <div className="dash-card">
          <div className="space-y-3 max-h-[600px] overflow-auto">
            {events.map((ev, i) => (
              <div key={i} className="flex items-start gap-3 p-4 bg-zinc-950/50 rounded-lg">
                <div className="flex flex-col gap-1.5 shrink-0">
                  <SeverityBadge severity={ev.severity} />
                  <Badge text={ev.category} variant="muted" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-zinc-200">{ev.title}</div>
                  <div className="text-xs text-zinc-400 mt-1">{ev.message}</div>
                  <div className="flex items-center gap-3 mt-2 text-xs text-zinc-500">
                    <span>{formatTimestamp(ev.timestamp)}</span>
                    {ev.workload && <span>Workload: {ev.workload}</span>}
                    <span>Source: {ev.source}</span>
                    {ev.acknowledged && <Badge text="ACK" variant="green" />}
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

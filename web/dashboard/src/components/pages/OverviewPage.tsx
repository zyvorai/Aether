import { useState, useEffect } from 'react';
import { LayoutDashboard, Activity, AlertTriangle, Calendar, Shield, Lock, Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import type { AppView } from '../../types/api';
import StatCard from '../StatCard';
import { SeverityBadge } from '../Badge';
import EmptyState from '../EmptyState';
import type { WorkloadResponse, EventSummary, HealthSummary, BackupInfo, SecretSummary, Event } from '../../types/api';

interface OverviewPageProps {
  onNavigate: (view: AppView) => void;
}

export default function OverviewPage({ onNavigate }: OverviewPageProps) {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [eventSummary, setEventSummary] = useState<EventSummary | null>(null);
  const [healthSummary, setHealthSummary] = useState<HealthSummary | null>(null);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [events, setEvents] = useState<Event[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      const [w, es, hs, b, s, ev] = await Promise.all([
        apiFetch<WorkloadResponse[]>('/workloads'),
        apiFetch<EventSummary>('/events/summary'),
        apiFetch<HealthSummary>('/orchestrator/summary'),
        apiFetch<BackupInfo[]>('/backups'),
        apiFetch<SecretSummary[]>('/secrets'),
        apiFetch<Event[]>('/events'),
      ]);
      setWorkloads(w ?? []);
      setEventSummary(es);
      setHealthSummary(hs);
      setBackups(b ?? []);
      setSecrets(s ?? []);
      setEvents(ev ?? []);
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

  const healthy = healthSummary?.healthy ?? 0;
  const degraded = (healthSummary?.degraded ?? 0) + (healthSummary?.unhealthy ?? 0);

  return (
    <div>
      <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
        <button onClick={() => onNavigate('workloads')} className="text-left">
          <StatCard title="Workloads" value={workloads.length} color="orange" icon={<LayoutDashboard size={18} />} />
        </button>
        <button onClick={() => onNavigate('health')} className="text-left">
          <StatCard title="Healthy" value={healthy} color="green" icon={<Activity size={18} />} />
        </button>
        <button onClick={() => onNavigate('health')} className="text-left">
          <StatCard title="Degraded" value={degraded} color="red" icon={<AlertTriangle size={18} />} />
        </button>
        <button onClick={() => onNavigate('events')} className="text-left">
          <StatCard title="Events" value={eventSummary?.total_events ?? 0} color="blue" icon={<Calendar size={18} />} />
        </button>
        <button onClick={() => onNavigate('backups')} className="text-left">
          <StatCard title="Backups" value={backups.length} color="purple" icon={<Shield size={18} />} />
        </button>
        <button onClick={() => onNavigate('secrets')} className="text-left">
          <StatCard title="Secrets" value={secrets.length} color="yellow" icon={<Lock size={18} />} />
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Recent Events */}
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Recent Events</h2>
          {events.length === 0 ? (
            <EmptyState icon={<Inbox size={48} />} title="No events" description="No events have been recorded yet" />
          ) : (
            <div className="space-y-3 max-h-96 overflow-auto">
              {events.slice(0, 20).map((ev, i) => (
                <div key={i} className="flex items-start gap-3 p-3 bg-zinc-950/50 rounded-lg">
                  <SeverityBadge severity={ev.severity} />
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium text-zinc-200 truncate">{ev.title}</div>
                    <div className="text-xs text-zinc-500 mt-0.5">{ev.message}</div>
                    <div className="text-xs text-zinc-600 mt-1">{formatTimestamp(ev.timestamp)}</div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Health Summary */}
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Health Summary</h2>
          {healthSummary ? (
            <div className="grid grid-cols-2 gap-4">
              <StatCard title="Healthy" value={healthSummary.healthy} color="green" />
              <StatCard title="Degraded" value={healthSummary.degraded} color="yellow" />
              <StatCard title="Unhealthy" value={healthSummary.unhealthy} color="red" />
              <StatCard title="Unknown" value={healthSummary.unknown} color="blue" />
              <StatCard title="Circuits Open" value={healthSummary.circuits_open} color="orange" />
            </div>
          ) : (
            <EmptyState icon={<Activity size={48} />} title="No health data" description="Health monitoring is not active" />
          )}
        </div>
      </div>
    </div>
  );
}

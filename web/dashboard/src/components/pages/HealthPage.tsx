import { useState, useEffect } from 'react';
import { Inbox, History } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import StatCard from '../StatCard';
import Badge, { RuntimeBadge } from '../Badge';
import Modal from '../Modal';
import CodeBlock from '../CodeBlock';
import EmptyState from '../EmptyState';
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

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function HealthPage() {
  const [summary, setSummary] = useState<HealthSummary | null>(null);
  const [workloads, setWorkloads] = useState<ManagedWorkload[]>([]);
  const [loading, setLoading] = useState(true);
  const [historyModal, setHistoryModal] = useState<{ name: string; data: HealthHistorySummary } | null>(null);
  const [historyLoading, setHistoryLoading] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      const [s, w] = await Promise.all([
        apiFetch<HealthSummary>('/orchestrator/summary'),
        apiFetch<ManagedWorkload[]>('/orchestrator/status'),
      ]);
      setSummary(s);
      setWorkloads(w ?? []);
      setLoading(false);
    }
    load();
  }, []);

  async function handleHealthHistory(name: string) {
    setHistoryLoading(name);
    const data = await apiFetch<HealthHistorySummary>(`/health/${name}`);
    setHistoryLoading(null);
    if (data) {
      setHistoryModal({ name, data });
      toast(`Health history for "${name}" loaded`, 'success');
    } else {
      toast(`Failed to load health history for "${name}"`, 'error');
    }
  }

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
          <StatCard title="Healthy" value={summary.healthy} color="green" />
          <StatCard title="Degraded" value={summary.degraded} color="yellow" />
          <StatCard title="Unhealthy" value={summary.unhealthy} color="red" />
          <StatCard title="Unknown" value={summary.unknown} color="blue" />
          <StatCard title="Circuits Open" value={summary.circuits_open} color="orange" />
        </div>
      )}

      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No managed workloads" description="No workloads are being monitored" />
      ) : (
        <div className="dash-card-flush">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Workload</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Runtime</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Health</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Circuit</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Restarts</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">History</th>
                </tr>
              </thead>
              <tbody>
                {workloads.map((w) => (
                  <tr key={w.name} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-4 font-medium text-zinc-200">{w.name}</td>
                    <td className="py-3 px-4"><RuntimeBadge runtime={w.runtime} /></td>
                    <td className="py-3 px-4">
                      <Badge text={w.health} variant={getHealthVariant(w.health)} />
                    </td>
                    <td className="py-3 px-4">
                      <Badge text={w.circuit} variant={getCircuitVariant(w.circuit)} />
                    </td>
                    <td className="py-3 px-4 text-sm text-zinc-300">{w.restart_count}</td>
                    <td className="py-3 px-4">
                      <button
                        onClick={() => handleHealthHistory(w.name)}
                        disabled={historyLoading === w.name}
                        className="p-1.5 text-zinc-400 hover:text-blue-400 hover:bg-blue-500/10 rounded transition-colors"
                        title="Health History"
                      >
                        {historyLoading === w.name ? (
                          <div className="animate-spin rounded-full h-3.5 w-3.5 border-b border-blue-400" />
                        ) : (
                          <History size={14} />
                        )}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Health History Modal */}
      <Modal
        isOpen={historyModal !== null}
        onClose={() => setHistoryModal(null)}
        title={`Health History: ${historyModal?.name ?? ''}`}
      >
        {historyModal && (
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="bg-zinc-950/50 rounded-lg p-3">
                <div className="text-xs text-zinc-500 mb-1">Total Checks</div>
                <div className="text-lg font-semibold text-zinc-100">{historyModal.data.total_checks}</div>
              </div>
              <div className="bg-zinc-950/50 rounded-lg p-3">
                <div className="text-xs text-zinc-500 mb-1">Ready Checks</div>
                <div className="text-lg font-semibold text-emerald-400">{historyModal.data.ready_checks}</div>
              </div>
              <div className="bg-zinc-950/50 rounded-lg p-3">
                <div className="text-xs text-zinc-500 mb-1">Uptime</div>
                <div className="text-lg font-semibold text-zinc-100">{historyModal.data.uptime_percent.toFixed(2)}%</div>
              </div>
              <div className="bg-zinc-950/50 rounded-lg p-3">
                <div className="text-xs text-zinc-500 mb-1">Last State</div>
                <div className="text-lg font-semibold text-zinc-100">{historyModal.data.last_state}</div>
              </div>
            </div>
            <div className="text-sm text-zinc-400">
              Last restart count: {historyModal.data.last_restart_count}
            </div>
            <CodeBlock title="Full History">{JSON.stringify(historyModal.data, null, 2)}</CodeBlock>
          </div>
        )}
      </Modal>
    </div>
  );
}

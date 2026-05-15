import { useState, useEffect } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import Modal from '../Modal';
import CodeBlock from '../CodeBlock';
import type { Environment } from '../../types/api';

function getTierVariant(tier: string): 'green' | 'yellow' | 'red' | 'blue' | 'muted' {
  const t = tier.toLowerCase();
  if (t === 'production' || t === 'prod') return 'red';
  if (t === 'staging') return 'yellow';
  if (t === 'development' || t === 'dev') return 'green';
  if (t === 'testing' || t === 'test') return 'blue';
  return 'muted';
}

export default function EnvsPage() {
  const [environments, setEnvironments] = useState<Environment[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedEnvironment, setSelectedEnvironment] = useState<Environment | null>(null);

  useEffect(() => {
    async function load() {
      const data = await apiFetch<Environment[]>('/environments');
      setEnvironments(data ?? []);
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
      {environments.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No environments" description="No environments have been configured" />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Tier</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Workloads</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Variables</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Updated</th>
                  <th className="text-right text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {environments.map((env) => (
                  <tr key={env.name} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-4 font-medium text-zinc-200">{env.name}</td>
                    <td className="py-3 px-4">
                      <Badge text={env.tier} variant={getTierVariant(env.tier)} />
                    </td>
                    <td className="py-3 px-4 text-sm text-zinc-300">{Object.keys(env.workloads).length}</td>
                    <td className="py-3 px-4 text-sm text-zinc-300">{Object.keys(env.variables).length}</td>
                    <td className="py-3 px-4 text-sm text-zinc-400">{formatTimestamp(env.updated_at)}</td>
                    <td className="py-3 px-4 text-right">
                      <button
                        onClick={() => setSelectedEnvironment(env)}
                        className="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-800"
                      >
                        Inspect
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal
        isOpen={selectedEnvironment !== null}
        onClose={() => setSelectedEnvironment(null)}
        title={selectedEnvironment ? `Environment: ${selectedEnvironment.name}` : 'Environment'}
      >
        {selectedEnvironment && (
          <div className="space-y-4">
            <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-[0.18em] text-slate-500">Tier</div>
                <div className="mt-2">
                  <Badge text={selectedEnvironment.tier} variant={getTierVariant(selectedEnvironment.tier)} />
                </div>
              </div>
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-[0.18em] text-slate-500">Workloads</div>
                <div className="mt-2 text-2xl font-semibold text-slate-100">{Object.keys(selectedEnvironment.workloads).length}</div>
              </div>
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-[0.18em] text-slate-500">Variables</div>
                <div className="mt-2 text-2xl font-semibold text-slate-100">{Object.keys(selectedEnvironment.variables).length}</div>
              </div>
            </div>

            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
              <div>
                <div className="mb-2 text-sm font-medium text-slate-200">Assigned Workloads</div>
                <CodeBlock title="json">{JSON.stringify(selectedEnvironment.workloads, null, 2)}</CodeBlock>
              </div>
              <div>
                <div className="mb-2 text-sm font-medium text-slate-200">Environment Variables</div>
                <CodeBlock title="json">{JSON.stringify(selectedEnvironment.variables, null, 2)}</CodeBlock>
              </div>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}

import { useState, useEffect } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
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
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

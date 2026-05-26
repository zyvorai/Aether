// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Modal from '../Modal';
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
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useState('');
  const [selectedEnvironment, setSelectedEnvironment] = useState<Environment | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<Environment[]>('/environments');
    if (!result.ok) {
      setLoadFailed(true);
      setEnvironments([]);
    } else {
      setEnvironments(result.data);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return environments;
    return environments.filter((env) => env.name.toLowerCase().includes(q) || env.tier.toLowerCase().includes(q));
  }, [environments, search]);

  if (loading && environments.length === 0 && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Environments unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search environments…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      {environments.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No environments" description="No environments have been configured" />
      ) : (
        <div className="dash-card overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-slate-800">
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Tier</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Workloads</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Variables</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Updated</th>
                  <th className="text-right text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((env) => (
                  <tr key={env.name} className="border-b border-slate-800/50 hover:bg-slate-800/30 transition-colors">
                    <td className="py-3 px-4 font-medium text-slate-200">{env.name}</td>
                    <td className="py-3 px-4">
                      <Badge text={env.tier} variant={getTierVariant(env.tier)} />
                    </td>
                    <td className="py-3 px-4 text-sm text-slate-300">{Object.keys(env.workloads).length}</td>
                    <td className="py-3 px-4 text-sm text-slate-300">{Object.keys(env.variables).length}</td>
                    <td className="py-3 px-4 text-sm text-slate-400">{formatTimestamp(env.updated_at)}</td>
                    <td className="py-3 px-4 text-right">
                      <button
                        type="button"
                        onClick={() => setSelectedEnvironment(env)}
                        className="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800"
                      >
                        Inspect
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {filtered.length === 0 && (
              <p className="text-sm text-slate-500 py-6 text-center">No environments match your search.</p>
            )}
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
                <div className="text-xs uppercase tracking-wider text-slate-500">Tier</div>
                <div className="mt-2">
                  <Badge text={selectedEnvironment.tier} variant={getTierVariant(selectedEnvironment.tier)} />
                </div>
              </div>
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Workloads</div>
                <div className="mt-2 text-2xl font-semibold text-slate-100">{Object.keys(selectedEnvironment.workloads).length}</div>
              </div>
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Variables</div>
                <div className="mt-2 text-2xl font-semibold text-slate-100">{Object.keys(selectedEnvironment.variables).length}</div>
              </div>
            </div>

            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
              <div>
                <div className="mb-2 text-sm font-medium text-slate-200">Assigned workloads</div>
                {Object.keys(selectedEnvironment.workloads).length === 0 ? (
                  <p className="text-sm text-slate-500">None</p>
                ) : (
                  <ul className="space-y-1 text-sm">
                    {Object.entries(selectedEnvironment.workloads).map(([name, value]) => (
                      <li key={name} className="flex justify-between gap-2 rounded-lg bg-slate-950/60 px-3 py-2 border border-slate-800">
                        <span className="text-slate-200">{name}</span>
                        <span className="text-slate-500 truncate">{String(value)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <div>
                <div className="mb-2 text-sm font-medium text-slate-200">Environment variables</div>
                {Object.keys(selectedEnvironment.variables).length === 0 ? (
                  <p className="text-sm text-slate-500">None</p>
                ) : (
                  <ul className="space-y-1 text-sm">
                    {Object.entries(selectedEnvironment.variables).map(([name, value]) => (
                      <li key={name} className="flex justify-between gap-2 rounded-lg bg-slate-950/60 px-3 py-2 border border-slate-800">
                        <span className="text-slate-400 font-mono">{name}</span>
                        <span className="text-slate-200 truncate">{String(value)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}

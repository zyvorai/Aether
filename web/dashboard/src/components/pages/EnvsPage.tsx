// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Link } from 'react-router';
import { Inbox } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { useAuth } from '../../contexts/AuthContext';
import { formatTimestamp } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Modal from '../Modal';
import type { Environment } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

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
  const [search, setSearch] = useQueryParam('q');
  const [envParam, setEnvParam] = useQueryParam('env');
  const [selectedEnvironment, setSelectedEnvironment] = useState<Environment | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState('');
  const [createTier, setCreateTier] = useState('development');
  const [promoteWorkload, setPromoteWorkload] = useState('');
  const [promoteFrom, setPromoteFrom] = useState('');
  const [promoteTo, setPromoteTo] = useState('');
  const [parityEnv1, setParityEnv1] = useState('');
  const [parityEnv2, setParityEnv2] = useState('');
  const [parityResult, setParityResult] = useState<string | null>(null);
  const [promoteResult, setPromoteResult] = useState<string | null>(null);
  const [mutating, setMutating] = useState(false);
  const { canMutate } = useAuth();

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

  useEffect(() => {
    if (!envParam || environments.length === 0) return;
    const match = environments.find((env) => env.name === envParam);
    if (match) {
      setSelectedEnvironment(match);
      setEnvParam('');
    }
  }, [envParam, environments, setEnvParam]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return environments;
    return environments.filter((env) => env.name.toLowerCase().includes(q) || env.tier.toLowerCase().includes(q));
  }, [environments, search]);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!canMutate || !createName.trim()) return;
    setMutating(true);
    const res = await apiPost('/environments', { name: createName.trim(), tier: createTier });
    setMutating(false);
    if (res.success) {
      setCreateOpen(false);
      setCreateName('');
      void load();
    }
  }

  async function handlePromote(e: React.FormEvent) {
    e.preventDefault();
    if (!canMutate || !promoteWorkload.trim() || !promoteFrom.trim() || !promoteTo.trim()) return;
    setMutating(true);
    const res = await apiPost('/environments/promote', {
      workload: promoteWorkload.trim(),
      from: promoteFrom.trim(),
      to: promoteTo.trim(),
    });
    setMutating(false);
    if (res.success) {
      setPromoteResult(`Promoted ${promoteWorkload.trim()} from ${promoteFrom.trim()} → ${promoteTo.trim()}`);
      toast(`Promoted ${promoteWorkload.trim()} from ${promoteFrom.trim()} → ${promoteTo.trim()}`, 'success');
      void load();
    } else {
      setPromoteResult(res.error ?? 'Promote failed');
      toast(res.error ?? 'Promote failed', 'error');
    }
  }

  async function handleParity(e: React.FormEvent) {
    e.preventDefault();
    if (!parityEnv1.trim() || !parityEnv2.trim()) return;
    setMutating(true);
    const result = await apiFetchSettled<{ reports?: unknown[] }>(
      `/environments/parity?env1=${encodeURIComponent(parityEnv1)}&env2=${encodeURIComponent(parityEnv2)}`,
    );
    setMutating(false);
    if (result.ok) {
      setParityResult(JSON.stringify(result.data, null, 2));
      toast(`Parity check: ${parityEnv1} vs ${parityEnv2}`, 'success');
    } else {
      setParityResult(result.error ?? 'Parity check failed');
      toast(result.error ?? 'Parity check failed', 'error');
    }
  }

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
        refreshing={loading || mutating}
        actions={
          canMutate ? (
            <button
              type="button"
              onClick={() => setCreateOpen(true)}
              className="rounded-xl bg-aether/20 border border-aether/40 px-4 py-2 text-sm text-aether hover:bg-aether/30"
            >
              Create environment
            </button>
          ) : null
        }
      />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        <div className="dash-card">
          <h3 className="text-sm font-semibold text-slate-200 mb-3">Promote workload</h3>
          <form onSubmit={(e) => void handlePromote(e)} className="space-y-3">
            <input
              type="text"
              value={promoteWorkload}
              onChange={(e) => setPromoteWorkload(e.target.value)}
              placeholder="Workload name"
              className="w-full rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm"
            />
            <div className="grid grid-cols-2 gap-3">
              <input
                type="text"
                value={promoteFrom}
                onChange={(e) => setPromoteFrom(e.target.value)}
                placeholder="From env"
                className="rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm"
              />
              <input
                type="text"
                value={promoteTo}
                onChange={(e) => setPromoteTo(e.target.value)}
                placeholder="To env"
                className="rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm"
              />
            </div>
            <button
              type="submit"
              disabled={!canMutate || mutating}
              data-testid="envs-promote-submit"
              className="rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800 disabled:opacity-50"
            >
              Promote
            </button>
          </form>
          {promoteResult ? (
            <p data-testid="envs-promote-result" className="mt-3 text-sm text-slate-400">{promoteResult}</p>
          ) : null}
        </div>
        <div className="dash-card">
          <h3 className="text-sm font-semibold text-slate-200 mb-3">Environment parity</h3>
          <form onSubmit={(e) => void handleParity(e)} className="space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <input
                type="text"
                value={parityEnv1}
                onChange={(e) => setParityEnv1(e.target.value)}
                placeholder="Env A"
                className="rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm"
              />
              <input
                type="text"
                value={parityEnv2}
                onChange={(e) => setParityEnv2(e.target.value)}
                placeholder="Env B"
                className="rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm"
              />
            </div>
            <button
              type="submit"
              data-testid="envs-parity-check"
              disabled={mutating}
              className="rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800 disabled:opacity-50"
            >
              Check parity
            </button>
          </form>
          {parityResult && (
            <>
              <pre data-testid="envs-parity-result" className="mt-3 text-xs text-slate-400 overflow-x-auto max-h-48">{parityResult}</pre>
              {parityResult.toLowerCase().includes('drift') || parityResult.toLowerCase().includes('mismatch') ? (
                <Link to={viewToPath('drift')} className="mt-2 inline-flex text-xs text-aether hover:underline" data-testid="envs-drift-link">
                  Open drift detection →
                </Link>
              ) : null}
            </>
          )}
        </div>
      </div>

      {environments.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No environments" description="No environments have been configured" />
      ) : (
        <div className="dash-card overflow-hidden" data-testid="envs-list">
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
                    <td className="py-3 px-4 text-sm text-slate-300">{Object.keys(env.workloads ?? {}).length}</td>
                    <td className="py-3 px-4 text-sm text-slate-300">{Object.keys(env.variables ?? {}).length}</td>
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
          <div className="space-y-4" data-testid="envs-inspect-modal">
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
                        <Link
                          to={pathWithQuery(viewToPath('workloads'), { workload: name })}
                          className="text-slate-200 hover:text-aether"
                        >
                          {name}
                        </Link>
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

      <Modal isOpen={createOpen} onClose={() => setCreateOpen(false)} title="Create environment">
        <form data-testid="envs-create-modal" onSubmit={(e) => void handleCreate(e)} className="space-y-4">
          <div>
            <label className="block text-xs text-slate-500 mb-1">Name</label>
            <input
              type="text"
              value={createName}
              onChange={(e) => setCreateName(e.target.value)}
              className="w-full rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm"
            />
          </div>
          <div>
            <label className="block text-xs text-slate-500 mb-1">Tier</label>
            <select
              value={createTier}
              onChange={(e) => setCreateTier(e.target.value)}
              className="w-full rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm"
            >
              <option value="development">development</option>
              <option value="staging">staging</option>
              <option value="production">production</option>
            </select>
          </div>
          <button
            type="submit"
            disabled={mutating || !createName.trim()}
            className="rounded-xl bg-aether/20 border border-aether/40 px-4 py-2 text-sm text-aether"
          >
            Create
          </button>
        </form>
      </Modal>
    </div>
  );
}

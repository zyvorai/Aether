// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Link } from 'react-router';
import { Inbox, Layers, Search } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam, useWorkloadOrSearchFilter } from '../../utils/urlState';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { useAuth } from '../../contexts/AuthContext';
import { formatTimestamp } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import CardGrid from '../CardGrid';
import EntityCard, { type EntityStatusTone } from '../EntityCard';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Modal from '../Modal';
import { SearchQueryContextBanner, WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { Environment } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

function getTierVariant(tier: string): 'green' | 'yellow' | 'red' | 'blue' | 'muted' {
  const t = tier.toLowerCase();
  if (t === 'production' || t === 'prod') return 'red';
  if (t === 'staging') return 'yellow';
  if (t === 'development' || t === 'dev') return 'green';
  if (t === 'qa' || t === 'test') return 'blue';
  return 'muted';
}

function getTierTone(tier: string): EntityStatusTone {
  const v = getTierVariant(tier);
  if (v === 'green') return 'green';
  if (v === 'yellow') return 'amber';
  if (v === 'red') return 'red';
  if (v === 'blue') return 'sky';
  return 'muted';
}

export default function EnvsPage({ refreshKey }: { refreshKey?: number } = {}) {
  const [environments, setEnvironments] = useState<Environment[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [hasLoadedOnce, setHasLoadedOnce] = useState(false);
  const [workloadParam] = useQueryParam('workload');
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const workloadFocus = workloadParam.trim();
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
    setHasLoadedOnce(true);
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (workloadFocus && !promoteWorkload) {
      setPromoteWorkload(workloadFocus);
    }
  }, [workloadFocus, promoteWorkload]);

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

  if (loading && !hasLoadedOnce && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Environments unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      {workloadFocus ? (
        <WorkloadContextBanner
          testId="envs-workload-banner"
          workload={workloadFocus}
          description="Environment promotion context"
        >
          <WorkloadScopedCrossLinks workload={workloadFocus} prefix="envs" showGitops />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: workloadFocus })}
            className="text-brand hover:underline"
            data-testid="envs-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('drift'), { workload: workloadFocus })}
            className="text-brand hover:underline"
            data-testid="envs-drift-link"
          >
            Drift →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: workloadFocus })}
            className="text-brand hover:underline"
            data-testid="envs-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: workloadFocus })}
            className="text-brand hover:underline"
            data-testid="envs-compose-link"
          >
            Compose →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: workloadFocus })}
            className="text-brand hover:underline"
            data-testid="envs-policy-link"
          >
            Policy →
          </Link>
        </WorkloadContextBanner>
      ) : (
        <SearchQueryContextBanner testId="envs-workload-context" query={search} entityLabel="environments">
          {search.trim() ? (
            <>
              <WorkloadScopedCrossLinks workload={search} prefix="envs" />
              {' · '}
              <Link
                to={pathWithQuery(viewToPath('editor'), { workload: search.trim() })}
                className="text-brand hover:underline"
                data-testid="envs-context-editor-link"
              >
                Editor →
              </Link>
              {' · '}
              <Link
                to={pathWithQuery(viewToPath('drift'), { workload: search.trim() })}
                className="text-brand hover:underline"
                data-testid="envs-context-drift-link"
              >
                Drift →
              </Link>
              {' · '}
              <Link
                to={pathWithQuery(viewToPath('secrets'), { workload: search.trim() })}
                className="text-brand hover:underline"
                data-testid="envs-context-secrets-link"
              >
                Secrets →
              </Link>
              {' · '}
              <Link
                to={pathWithQuery(viewToPath('compose'), { workload: search.trim() })}
                className="text-brand hover:underline"
                data-testid="envs-context-compose-link"
              >
                Compose →
              </Link>
              {' · '}
              <Link
                to={pathWithQuery(viewToPath('policy'), { workload: search.trim() })}
                className="text-brand hover:underline"
                data-testid="envs-context-policy-link"
              >
                Policy →
              </Link>
            </>
          ) : null}
        </SearchQueryContextBanner>
      )}
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
              className="btn-primary"
            >
              Create environment
            </button>
          ) : null
        }
      />

      <section className="overview-section-shell mb-6 p-6 sm:p-8">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        <div className="glass-panel-card">
          <h3 className="text-sm font-semibold text-ink mb-3">Promote workload</h3>
          <form onSubmit={(e) => void handlePromote(e)} className="space-y-3">
            <input
              type="text"
              value={promoteWorkload}
              onChange={(e) => setPromoteWorkload(e.target.value)}
              placeholder="Workload name"
              data-testid="envs-promote-workload"
              className="glass-input"
            />
            <div className="grid grid-cols-2 gap-3">
              <input
                type="text"
                value={promoteFrom}
                onChange={(e) => setPromoteFrom(e.target.value)}
                placeholder="From env"
                className="glass-input"
              />
              <input
                type="text"
                value={promoteTo}
                onChange={(e) => setPromoteTo(e.target.value)}
                placeholder="To env"
                className="glass-input"
              />
            </div>
            <button
              type="submit"
              disabled={!canMutate || mutating}
              data-testid="envs-promote-submit"
              className="rounded-xl border glass-divider px-4 py-2 text-sm text-ink glass-inset-hover disabled:opacity-50"
            >
              Promote
            </button>
          </form>
          {promoteResult ? (
            <p data-testid="envs-promote-result" className="mt-3 text-sm text-ink-2">{promoteResult}</p>
          ) : null}
        </div>
        <div className="glass-panel-card">
          <h3 className="text-sm font-semibold text-ink mb-3">Environment parity</h3>
          <form onSubmit={(e) => void handleParity(e)} className="space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <input
                type="text"
                value={parityEnv1}
                onChange={(e) => setParityEnv1(e.target.value)}
                placeholder="Env A"
                className="glass-input"
              />
              <input
                type="text"
                value={parityEnv2}
                onChange={(e) => setParityEnv2(e.target.value)}
                placeholder="Env B"
                className="glass-input"
              />
            </div>
            <button
              type="submit"
              data-testid="envs-parity-check"
              disabled={mutating}
              className="rounded-xl border glass-divider px-4 py-2 text-sm text-ink glass-inset-hover disabled:opacity-50"
            >
              Check parity
            </button>
          </form>
          {parityResult && (
            <>
              <pre data-testid="envs-parity-result" className="mt-3 text-xs text-ink-2 overflow-x-auto max-h-48">{parityResult}</pre>
              {parityResult.toLowerCase().includes('drift') || parityResult.toLowerCase().includes('mismatch') ? (
                <Link to={viewToPath('drift')} className="mt-2 inline-flex text-xs text-brand hover:underline" data-testid="envs-drift-link">
                  Open drift detection →
                </Link>
              ) : null}
            </>
          )}
        </div>
      </div>

      {environments.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No environments" description="No environments have been configured" />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No matching environments" description="Try adjusting your search." />
      ) : (
        <CardGrid columns="compact" testId="envs-list">
          {filtered.map((env, i) => (
            <EntityCard
              key={env.name}
              index={i}
              testId={`env-card-${env.name}`}
              icon={<Layers size={18} />}
              statusTone={getTierTone(env.tier)}
              title={env.name}
              subtitle={formatTimestamp(env.updated_at)}
              badge={<Badge text={env.tier} variant={getTierVariant(env.tier)} />}
              onClick={() => setSelectedEnvironment(env)}
              body={
                <div className="flex flex-wrap gap-1.5">
                  <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-ink-2">
                    {Object.keys(env.workloads ?? {}).length} workloads
                  </span>
                  <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-ink-2">
                    {Object.keys(env.variables ?? {}).length} vars
                  </span>
                </div>
              }
              footer={
                <button
                  type="button"
                  onClick={() => setSelectedEnvironment(env)}
                  className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-ink-2 transition hover:bg-white/5 hover:text-ink"
                >
                  <Search size={13} />
                  Inspect
                </button>
              }
            />
          ))}
        </CardGrid>
      )}

      </section>

      <Modal
        isOpen={selectedEnvironment !== null}
        onClose={() => setSelectedEnvironment(null)}
        title={selectedEnvironment ? `Environment: ${selectedEnvironment.name}` : 'Environment'}
      >
        {selectedEnvironment && (
          <div className="space-y-4" data-testid="envs-inspect-modal">
            <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-ink-3">Tier</div>
                <div className="mt-2">
                  <Badge text={selectedEnvironment.tier} variant={getTierVariant(selectedEnvironment.tier)} />
                </div>
              </div>
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-ink-3">Workloads</div>
                <div className="mt-2 text-2xl font-semibold text-ink">{Object.keys(selectedEnvironment.workloads).length}</div>
              </div>
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-ink-3">Variables</div>
                <div className="mt-2 text-2xl font-semibold text-ink">{Object.keys(selectedEnvironment.variables).length}</div>
              </div>
            </div>

            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
              <div>
                <div className="mb-2 text-sm font-medium text-ink">Assigned workloads</div>
                {Object.keys(selectedEnvironment.workloads).length === 0 ? (
                  <p className="text-sm text-ink-3">None</p>
                ) : (
                  <ul className="space-y-1 text-sm">
                    {Object.entries(selectedEnvironment.workloads).map(([name, value]) => (
                      <li key={name} className="flex justify-between gap-2 glass-table-row rounded-lg px-3 py-2">
                        <Link
                          to={pathWithQuery(viewToPath('workloads'), { workload: name })}
                          className="text-ink hover:text-brand"
                        >
                          {name}
                        </Link>
                        <span className="text-ink-3 truncate">{String(value)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <div>
                <div className="mb-2 text-sm font-medium text-ink">Environment variables</div>
                {Object.keys(selectedEnvironment.variables).length === 0 ? (
                  <p className="text-sm text-ink-3">None</p>
                ) : (
                  <ul className="space-y-1 text-sm">
                    {Object.entries(selectedEnvironment.variables).map(([name, value]) => (
                      <li key={name} className="flex justify-between gap-2 glass-table-row rounded-lg px-3 py-2">
                        <span className="text-ink-2 font-mono">{name}</span>
                        <span className="text-ink truncate">{String(value)}</span>
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
            <label className="block text-xs text-ink-3 mb-1">Name</label>
            <input
              type="text"
              value={createName}
              onChange={(e) => setCreateName(e.target.value)}
              className="glass-input"
            />
          </div>
          <div>
            <label className="block text-xs text-ink-3 mb-1">Tier</label>
            <select
              value={createTier}
              onChange={(e) => setCreateTier(e.target.value)}
              className="glass-input"
            >
              <option value="development">development</option>
              <option value="staging">staging</option>
              <option value="production">production</option>
            </select>
          </div>
          <button
            type="submit"
            disabled={mutating || !createName.trim()}
            className="btn-primary disabled:opacity-50"
          >
            Create
          </button>
        </form>
      </Modal>
    </div>
  );
}

// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Link } from 'react-router';
import { Inbox } from 'lucide-react';
import { apiFetch, apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam, useWorkloadOrSearchFilter } from '../../utils/urlState';
import { useAuth } from '../../contexts/AuthContext';
import PageToolbar from '../PageToolbar';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import StatCard from '../StatCard';
import { SearchQueryContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { WorkloadResponse, SlaTarget } from '../../types/api';

export default function SLAPage() {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [slaData, setSlaData] = useState<Record<string, SlaTarget | null>>({});
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [addWorkload, setAddWorkload] = useState('');
  const [addTier, setAddTier] = useState('standard');
  const [adding, setAdding] = useState(false);
  const { canMutate } = useAuth();

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<WorkloadResponse[]>('/workloads');
    if (!result.ok) {
      setLoadFailed(true);
      setWorkloads([]);
      setSlaData({});
      setLoading(false);
      return;
    }
    const wl = result.data;
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
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = workloads.filter((w) => w.name.toLowerCase().includes(search.toLowerCase()));
  const slaConfiguredCount = workloads.filter((w) => slaData[w.name]).length;
  const complianceGapCount = workloads.length - slaConfiguredCount;

  async function handleAddSla(e: React.FormEvent) {
    e.preventDefault();
    if (!canMutate || !addWorkload.trim()) return;
    setAdding(true);
    const res = await apiPost(`/sla`, { workload: addWorkload.trim(), tier: addTier });
    setAdding(false);
    if (res.success) {
      setAddWorkload('');
      void load();
    }
  }

  if (loading && workloads.length === 0 && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="SLA data unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <SearchQueryContextBanner testId="sla-workload-context" query={search} entityLabel="SLA workloads">
        <WorkloadScopedCrossLinks workload={search} prefix="sla-banner" />
        {search.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="sla-editor-link"
            >
              Editor →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('drift'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="sla-drift-link"
            >
              Drift →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="sla-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('alerts'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="sla-context-alerts-link"
            >
              Alerts →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="sla-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('copilot'), { workload: search.trim(), q: `SLA status for ${search.trim()}` })}
              className="text-aether hover:underline"
              data-testid="sla-context-copilot-link"
            >
              Copilot →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('platform'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="sla-context-platform-link"
            >
              Platform →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="sla-context-openapi-link"
            >
              OpenAPI →
            </Link>
          </>
        ) : null}
      </SearchQueryContextBanner>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Filter workloads…"
        onRefresh={() => void load()}
        refreshing={loading || adding}
      />

      {canMutate && (
        <div className="glass-panel-card mb-6">
          <h3 className="text-sm font-semibold text-slate-200 mb-3">Add SLA target</h3>
          <form onSubmit={(e) => void handleAddSla(e)} className="flex flex-wrap gap-3" data-testid="sla-add-form">
            <input
              type="text"
              value={addWorkload}
              onChange={(e) => setAddWorkload(e.target.value)}
              placeholder="Workload name"
              className="glass-input min-w-[160px]"
            />
            <select
              value={addTier}
              onChange={(e) => setAddTier(e.target.value)}
              className="glass-input"
            >
              <option value="standard">standard</option>
              <option value="high-availability">high-availability</option>
              <option value="best-effort">best-effort</option>
            </select>
            <button
              type="submit"
              disabled={adding || !addWorkload.trim()}
              className="btn-primary disabled:opacity-50"
            >
              {adding ? 'Adding…' : 'Add target'}
            </button>
          </form>
          <Link to={pathWithQuery(viewToPath('events'), { category: 'sla' })} className="mt-3 inline-flex text-xs text-aether hover:underline">
            SLA events →
          </Link>
          <Link to={viewToPath('health')} className="mt-3 ml-4 inline-flex text-xs text-aether hover:underline">
            Open health monitor →
          </Link>
          <Link to={viewToPath('scheduler')} className="mt-3 ml-4 inline-flex text-xs text-aether hover:underline" data-testid="sla-scheduler-link">
            Placement scheduler →
          </Link>
        </div>
      )}

      {workloads.length > 0 && (
        <section className="overview-section-shell mb-6 p-6 sm:p-8">
        <div className="grid grid-cols-2 lg:grid-cols-3 gap-4">
          <StatCard title="With SLA" value={slaConfiguredCount} color="green" />
          <Link to={pathWithQuery(viewToPath('events'), { category: 'sla' })} className="text-left" data-testid="sla-breach-stat">
            <StatCard title="Compliance gaps" value={complianceGapCount} color={complianceGapCount > 0 ? 'red' : 'blue'} />
          </Link>
        </div>
        </section>
      )}

      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No workloads" description="Deploy a workload to configure SLA targets" />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No matches" description="Try a different search term" />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6" data-testid="sla-workload-grid">
          {filtered.map((w) => {
            const sla = slaData[w.name];
            return (
              <div key={w.name} className="glass-panel-card">
                <h2 className="text-lg font-semibold text-slate-100 mb-4">
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: w.name })}
                    className="hover:text-aether"
                  >
                    {w.name}
                  </Link>
                </h2>
                {sla ? (
                  <dl className="space-y-3 text-sm">
                    <div className="flex justify-between">
                      <dt className="text-slate-400">Uptime target</dt>
                      <dd className="text-emerald-400 font-medium">{sla.uptime_target_pct}%</dd>
                    </div>
                    {sla.max_latency_ms !== null && (
                      <div className="flex justify-between">
                        <dt className="text-slate-400">Max latency</dt>
                        <dd className="text-slate-200">{sla.max_latency_ms}ms</dd>
                      </div>
                    )}
                    {sla.max_error_rate_pct !== null && (
                      <div className="flex justify-between">
                        <dt className="text-slate-400">Max error rate</dt>
                        <dd className="text-slate-200">{sla.max_error_rate_pct}%</dd>
                      </div>
                    )}
                    {sla.max_restarts_per_day !== null && (
                      <div className="flex justify-between">
                        <dt className="text-slate-400">Max restarts/day</dt>
                        <dd className="text-slate-200">{sla.max_restarts_per_day}</dd>
                      </div>
                    )}
                  </dl>
                ) : (
                  <p className="text-sm text-slate-500">No SLA configured</p>
                )}
                <div className="mt-4 flex flex-wrap gap-3 text-xs">
                  <Link
                    to={pathWithQuery(viewToPath('health'), { workload: w.name })}
                    className="text-aether hover:underline"
                    data-testid={`sla-health-link-${w.name}`}
                  >
                    Health →
                  </Link>
                  <Link
                    to={pathWithQuery(viewToPath('events'), { workload: w.name, category: 'sla' })}
                    className="text-aether hover:underline"
                    data-testid={`sla-events-link-${w.name}`}
                  >
                    SLA events →
                  </Link>
                  <Link
                    to={pathWithQuery(viewToPath('alerts'), { workload: w.name })}
                    className="text-aether hover:underline"
                    data-testid={`sla-alerts-link-${w.name}`}
                  >
                    Alerts →
                  </Link>
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: w.name, tab: 'trust' })}
                    className="text-aether hover:underline"
                    data-testid={`sla-trust-link-${w.name}`}
                  >
                    Trust →
                  </Link>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

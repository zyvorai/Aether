// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { withAuroraPage } from '../layout/AuroraPage';
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
import { SearchQueryContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { WorkloadResponse, SlaTarget } from '../../types/api';

function SLAPage({ refreshKey }: { refreshKey?: number } = {}) {
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
    // Fetch in small concurrent batches rather than firing one request per workload
    // at once — with dozens of workloads that starves the browser's per-origin
    // connection pool and stalls the whole page for many seconds.
    const CONCURRENCY = 8;
    for (let i = 0; i < wl.length; i += CONCURRENCY) {
      const batch = wl.slice(i, i + CONCURRENCY);
      await Promise.all(
        batch.map(async (w) => {
          const sla = await apiFetch<SlaTarget>(`/sla/${w.name}`);
          results[w.name] = sla;
        })
      );
    }
    setSlaData(results);
    setLoading(false);
  }, [refreshKey]);

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
              className="text-primary hover:underline"
              data-testid="sla-editor-link"
            >
              Editor →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('drift'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="sla-drift-link"
            >
              Drift →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="sla-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('alerts'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="sla-context-alerts-link"
            >
              Alerts →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="sla-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('zyra'), { workload: search.trim(), q: `SLA status for ${search.trim()}` })}
              className="text-primary hover:underline"
              data-testid="sla-context-copilot-link"
            >
              Copilot →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('platform'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="sla-context-platform-link"
            >
              Platform →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: search.trim() })}
              className="text-primary hover:underline"
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
        <div className="glass mb-6">
          <h3 className="text-sm font-semibold text-foreground mb-3">Add SLA target</h3>
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
          <Link to={pathWithQuery(viewToPath('events'), { category: 'sla' })} className="mt-3 inline-flex text-xs text-primary hover:underline">
            SLA events →
          </Link>
          <Link to={viewToPath('health')} className="mt-3 ml-4 inline-flex text-xs text-primary hover:underline">
            Open health monitor →
          </Link>
          <Link to={viewToPath('scheduler')} className="mt-3 ml-4 inline-flex text-xs text-primary hover:underline" data-testid="sla-scheduler-link">
            Placement scheduler →
          </Link>
        </div>
      )}

      {workloads.length > 0 && (
        <section className="mb-10">
        <div className="flex flex-wrap gap-x-10 gap-y-4">
          <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{slaConfiguredCount}</div>With SLA</div>
          <Link to={pathWithQuery(viewToPath('events'), { category: 'sla' })} className="text-left text-sm text-muted hover:text-foreground" data-testid="sla-breach-stat">
            <div className="text-2xl font-semibold tabular-nums text-foreground">{complianceGapCount}</div>Compliance gaps
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
              <div key={w.name} className="glass">
                <h2 className="text-lg font-semibold text-foreground mb-4">
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: w.name })}
                    className="hover:text-primary"
                  >
                    {w.name}
                  </Link>
                </h2>
                {sla ? (
                  <dl className="space-y-3 text-sm">
                    <div className="flex justify-between">
                      <dt className="text-muted">Uptime target</dt>
                      <dd className="text-success font-medium">{sla.uptime_target_pct}%</dd>
                    </div>
                    {sla.max_latency_ms !== null && (
                      <div className="flex justify-between">
                        <dt className="text-muted">Max latency</dt>
                        <dd className="text-foreground">{sla.max_latency_ms}ms</dd>
                      </div>
                    )}
                    {sla.max_error_rate_pct !== null && (
                      <div className="flex justify-between">
                        <dt className="text-muted">Max error rate</dt>
                        <dd className="text-foreground">{sla.max_error_rate_pct}%</dd>
                      </div>
                    )}
                    {sla.max_restarts_per_day !== null && (
                      <div className="flex justify-between">
                        <dt className="text-muted">Max restarts/day</dt>
                        <dd className="text-foreground">{sla.max_restarts_per_day}</dd>
                      </div>
                    )}
                  </dl>
                ) : (
                  <p className="text-sm text-subtle">No SLA configured</p>
                )}
                <div className="mt-4 flex flex-wrap gap-3 text-xs">
                  <Link
                    to={pathWithQuery(viewToPath('health'), { workload: w.name })}
                    className="text-primary hover:underline"
                    data-testid={`sla-health-link-${w.name}`}
                  >
                    Health →
                  </Link>
                  <Link
                    to={pathWithQuery(viewToPath('events'), { workload: w.name, category: 'sla' })}
                    className="text-primary hover:underline"
                    data-testid={`sla-events-link-${w.name}`}
                  >
                    SLA events →
                  </Link>
                  <Link
                    to={pathWithQuery(viewToPath('alerts'), { workload: w.name })}
                    className="text-primary hover:underline"
                    data-testid={`sla-alerts-link-${w.name}`}
                  >
                    Alerts →
                  </Link>
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: w.name, tab: 'trust' })}
                    className="text-primary hover:underline"
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

export default withAuroraPage('sla', SLAPage);

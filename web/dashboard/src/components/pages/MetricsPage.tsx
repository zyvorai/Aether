import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Link, useNavigate } from 'react-router';
import { Download, ExternalLink } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useWorkloadOrSearchFilter } from '../../utils/urlState';
import { apiFetchSettled, apiTextSettled, apiFetch } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import StatCard from '../StatCard';
import CodeBlock from '../CodeBlock';
import { SearchQueryContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { ObservabilitySummary } from '../../types/api';
import { SectionHeader } from '../layout/SectionHeader';

interface ChargebackReport {
  totalMonthlyUsd: number;
  totalSpotMonthlyUsd: number;
  tco36MonthsUsd: number;
  pricingSource: string;
  region: string;
  lines: { workload: string; owner: string; project: string; monthlyUsd: number }[];
}

function MetricsPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [metrics, setMetrics] = useState('');
  const [summary, setSummary] = useState<ObservabilitySummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [grafanaUrl, setGrafanaUrl] = useState<string | null>(null);
  const [prometheusUrl, setPrometheusUrl] = useState<string | null>(null);
  const [chargeback, setChargeback] = useState<ChargebackReport | null>(null);
  const [promQuery, setPromQuery] = useState('up');
  const [promResult, setPromResult] = useState<string | null>(null);
  const [promQuerying, setPromQuerying] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [metricsRes, serverRes, chargebackRes, summaryRes] = await Promise.all([
      apiTextSettled('/metrics'),
      apiFetchSettled<Record<string, unknown>>('/server'),
      apiFetchSettled<ChargebackReport>('/cost/chargeback?provider=aws'),
      apiFetchSettled<ObservabilitySummary>('/observability/summary'),
    ]);
    if (!metricsRes.ok) {
      setLoadFailed(true);
      setMetrics('');
      setSummary(null);
      setGrafanaUrl(null);
      setPrometheusUrl(null);
      setChargeback(null);
    } else {
      setMetrics(metricsRes.data);
      setSummary(summaryRes.ok ? summaryRes.data : null);
      if (serverRes.ok) {
        const integrations = serverRes.data?.integrations as Record<string, unknown> | undefined;
        const gUrl = integrations?.grafana_url;
        const pUrl = integrations?.prometheus_url;
        setGrafanaUrl(typeof gUrl === 'string' ? gUrl : null);
        setPrometheusUrl(typeof pUrl === 'string' ? pUrl : null);
      } else {
        setGrafanaUrl(null);
        setPrometheusUrl(null);
      }
      setChargeback(chargebackRes.ok ? chargebackRes.data : null);
    }
    setLoading(false);
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  const filteredMetrics = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return metrics;
    return metrics
      .split('\n')
      .filter((line) => line.startsWith('#') || line.toLowerCase().includes(q))
      .join('\n');
  }, [metrics, search]);

  const lineCount = filteredMetrics.split('\n').filter((l) => l && !l.startsWith('#')).length;
  const runtimeEntries = Object.entries(summary?.workloads_running ?? {});

  function downloadChargebackCsv() {
    if (!chargeback?.lines.length) return;
    const header = 'workload,owner,project,monthly_usd';
    const rows = chargeback.lines.map(
      (l) =>
        `"${l.workload}","${l.owner.replace(/"/g, '""')}","${l.project.replace(/"/g, '""')}",${l.monthlyUsd.toFixed(2)}`,
    );
    const blob = new Blob([[header, ...rows].join('\n')], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'aether-chargeback.csv';
    a.click();
    URL.revokeObjectURL(url);
  }

  async function runPromQuery(e: React.FormEvent) {
    e.preventDefault();
    const q = promQuery.trim();
    if (!q) return;
    setPromQuerying(true);
    setPromResult(null);
    const data = await apiFetch<unknown>(`/observability/prometheus/query?query=${encodeURIComponent(q)}`);
    setPromQuerying(false);
    setPromResult(data ? JSON.stringify(data, null, 2) : 'Query failed or Prometheus not configured');
  }

  if (loading && !metrics && !loadFailed) {
    return <PageLoading rows={6} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Metrics unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <SearchQueryContextBanner testId="metrics-workload-context" query={search} entityLabel="metrics">
        <WorkloadScopedCrossLinks workload={search} prefix="metrics" />
        {search.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('cost'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="metrics-cost-link"
            >
              Cost →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('platform'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="metrics-platform-link"
            >
              Platform →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('drift'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="metrics-drift-link"
            >
              Drift →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('intelligence'), { workload: search.trim(), tab: 'predictions' })}
              className="text-primary hover:underline"
              data-testid="metrics-context-intelligence-link"
            >
              Intelligence →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('alerts'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="metrics-context-alerts-link"
            >
              Alerts →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('scheduler'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="metrics-context-scheduler-link"
            >
              Scheduler →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="metrics-context-openapi-link"
            >
              OpenAPI →
            </Link>
          </>
        ) : null}
      </SearchQueryContextBanner>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search metric names…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      {!prometheusUrl && !summary?.prometheus_configured ? (
        <div
          data-testid="metrics-prom-setup-banner"
          className="glass-context-banner mb-6 flex flex-wrap items-center justify-between gap-3 text-sm text-muted"
        >
          <span>Prometheus is not linked — set Prometheus URL on Platform &amp; HA for live query explorer and external links.</span>
          <button
            type="button"
            onClick={() => navigate(viewToPath('platform'))}
            className="btn-secondary text-xs"
          >
            Open Platform
          </button>
        </div>
      ) : null}

      {summary && (
        <section className="glass mb-6 p-6 sm:p-8">
          <SectionHeader
            label="Observability"
            title="Platform metrics"
            description="API traffic, migrations, and cluster resource signals"
          />
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
          <StatCard title="API requests" value={Math.round(summary.api_http_requests_total)} color="blue" />
          <StatCard title="Migrations" value={Math.round(summary.migrations_total)} color="primary" />
          <StatCard title="Rollbacks" value={Math.round(summary.migration_rollbacks_total)} color="red" />
          <StatCard
            title="Cluster CPU"
            value={summary.cluster_metrics ? `${summary.cluster_metrics.total_cpu_millicores}m` : '—'}
            color="green"
          />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="glass">
            <h2 className="panel-title mb-3">Workloads by runtime</h2>
            {runtimeEntries.length > 0 ? (
              <dl className="space-y-2 text-sm">
                {runtimeEntries.map(([runtime, count]) => (
                  <div key={runtime} className="flex justify-between">
                    <dt className="text-muted">{runtime}</dt>
                    <dd className="text-foreground font-mono">{Math.round(count)}</dd>
                  </div>
                ))}
              </dl>
            ) : (
              <p className="text-sm text-subtle">No running workload gauges reported yet.</p>
            )}
          </div>
          <div className="glass">
            <h2 className="panel-title mb-3">Cluster & Cilium</h2>
            {summary.cluster_metrics ? (
              <dl className="space-y-2 text-sm mb-4">
                <div className="flex justify-between">
                  <dt className="text-muted">Scope</dt>
                  <dd className="text-foreground">{summary.cluster_metrics.scope}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted">Pods measured</dt>
                  <dd className="text-foreground">{summary.cluster_metrics.pod_count}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted">Memory</dt>
                  <dd className="text-foreground">{summary.cluster_metrics.total_memory_mib} Mi</dd>
                </div>
              </dl>
            ) : (
              <p className="text-sm text-subtle mb-4">Pass <code>cluster=</code> to <code>/api/observability/summary</code> for cluster metrics.</p>
            )}
            {summary.cilium ? (
              <dl className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <dt className="text-muted">CNI</dt>
                  <dd className="text-foreground">{summary.cilium.cni}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted">Egress mode</dt>
                  <dd className="text-foreground">{summary.cilium.egress_mode}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted">metrics-server</dt>
                  <dd className="text-foreground">{summary.cilium.metrics_server ? 'ok' : 'missing'}</dd>
                </div>
              </dl>
            ) : (
              <p className="text-sm text-subtle">Cilium status unavailable from active cluster.</p>
            )}
            <div className="mt-4 flex flex-wrap gap-2">
              <button
                type="button"
                onClick={() => navigate(pathWithQuery(viewToPath('clusters'), { tab: 'network' }))}
                className="rounded-xl border glass-divider px-3 py-1.5 text-xs text-muted hover:border-primary/40 hover:text-primary"
                data-testid="metrics-cluster-browser-link"
              >
                Open cluster browser (network)
              </button>
            </div>
            {summary.prometheus_configured && (
              <p className="mt-3 text-xs text-subtle">
                Prometheus linked — whitelisted queries via <code>/api/observability/prometheus/query</code>
              </p>
            )}
          </div>
        </div>
        </section>
      )}

      {chargeback && (
        <div className="glass mb-6" data-testid="metrics-chargeback-panel">
          <div className="flex flex-wrap items-center justify-between gap-3 mb-3">
            <h2 className="text-lg font-semibold text-foreground">Chargeback (showback)</h2>
            <div className="flex items-center gap-3">
              <button
                type="button"
                data-testid="metrics-fleet-link"
                onClick={() =>
                  navigate(
                    search.trim()
                      ? pathWithQuery(viewToPath('fleet'), { workload: search.trim() })
                      : viewToPath('fleet'),
                  )
                }
                className="text-xs text-primary hover:underline"
              >
                Fleet overview →
              </button>
              <button
                type="button"
                data-testid="metrics-cost-estimator-link"
                onClick={() =>
                  navigate(
                    search.trim()
                      ? pathWithQuery(viewToPath('cost'), { workload: search.trim() })
                      : viewToPath('cost'),
                  )
                }
                className="text-xs text-primary hover:underline"
              >
                Cost estimator →
              </button>
              <button
                type="button"
                data-testid="metrics-workloads-link"
                onClick={() =>
                  navigate(
                    search.trim()
                      ? pathWithQuery(viewToPath('workloads'), { workload: search.trim() })
                      : viewToPath('workloads'),
                  )
                }
                className="text-xs text-primary hover:underline"
              >
                All workloads →
              </button>
              {chargeback.lines.length > 0 ? (
              <button
                type="button"
                data-testid="metrics-chargeback-export"
                onClick={downloadChargebackCsv}
                className="inline-flex items-center gap-1.5 rounded-xl border glass-divider px-3 py-1.5 text-xs text-muted hover:border-primary/40"
              >
                <Download className="w-3.5 h-3.5" />
                Export CSV
              </button>
            ) : null}
            </div>
          </div>
          <p className="text-sm text-muted mb-4">
            {chargeback.pricingSource} pricing · {chargeback.region} · fleet ${chargeback.totalMonthlyUsd.toFixed(2)}/mo
            · spot ${chargeback.totalSpotMonthlyUsd.toFixed(2)}/mo · 36-mo TCO ${chargeback.tco36MonthsUsd.toFixed(0)}
          </p>
          {chargeback.lines.length > 0 ? (
            <div className="glass-table-shell overflow-x-auto">
              <table className="w-full text-sm text-left text-muted">
                <thead className="text-xs uppercase text-subtle glass-divider-b">
                  <tr>
                    <th className="py-2 pr-4">Workload</th>
                    <th className="py-2 pr-4">Owner</th>
                    <th className="py-2 pr-4">Project</th>
                    <th className="py-2">$/mo</th>
                  </tr>
                </thead>
                <tbody>
                  {chargeback.lines.slice(0, 12).map((line) => (
                    <tr key={line.workload} className="glass-divider-b/80">
                      <td className="py-2 pr-4 font-mono text-xs">
                        <button
                          type="button"
                          onClick={() =>
                            navigate(pathWithQuery(viewToPath('workloads'), { workload: line.workload }))
                          }
                          className="text-primary hover:underline"
                        >
                          {line.workload}
                        </button>
                      </td>
                      <td className="py-2 pr-4">{line.owner}</td>
                      <td className="py-2 pr-4">{line.project}</td>
                      <td className="py-2">${line.monthlyUsd.toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p className="text-sm text-subtle">No deployed workloads with readable specs for chargeback.</p>
          )}
        </div>
      )}

      {(grafanaUrl || prometheusUrl) && (
        <div className="glass mb-6 flex flex-wrap items-center justify-between gap-4" data-testid="metrics-observability-panel">
          <p className="text-sm text-muted">External observability stack linked to this API.</p>
          <div className="flex flex-wrap gap-3">
            {grafanaUrl && (
              <a
                href={grafanaUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 rounded-xl border border-primary/40 bg-primary/10 px-4 py-2 text-sm text-primary hover:bg-primary/20"
              >
                Open Grafana <ExternalLink size={14} />
              </a>
            )}
            {prometheusUrl && (
              <a
                href={prometheusUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 rounded-xl border glass-divider px-4 py-2 text-sm text-foreground glass-inset-hover"
              >
                Prometheus <ExternalLink size={14} />
              </a>
            )}
          </div>
        </div>
      )}

      {summary?.prometheus_configured && (
        <div className="glass mb-6">
          <h2 className="text-lg font-semibold text-foreground mb-3">Prometheus query explorer</h2>
          <p className="text-sm text-subtle mb-4">Instant queries via the whitelisted API proxy.</p>
          <form onSubmit={(e) => void runPromQuery(e)} className="flex flex-wrap gap-3 mb-4" data-testid="metrics-prometheus-query">
            <input
              type="text"
              value={promQuery}
              onChange={(e) => setPromQuery(e.target.value)}
              placeholder="e.g. aether_workloads_running or up"
              className="flex-1 min-w-[200px] glass-input font-mono text-foreground"
            />
            <button
              type="submit"
              disabled={promQuerying}
              className="btn-primary disabled:opacity-50"
            >
              {promQuerying ? 'Querying…' : 'Run query'}
            </button>
          </form>
          {promResult && (
            <CodeBlock title="prometheus-query">{promResult}</CodeBlock>
          )}
        </div>
      )}

      <div className="glass">
        <div className="flex items-center justify-between mb-4">
          <h2 className="panel-title">Prometheus metrics</h2>
          <span className="text-xs text-subtle">{lineCount} metric lines</span>
        </div>
        <CodeBlock title="prometheus">{filteredMetrics || 'No metrics match your search.'}</CodeBlock>
      </div>

      {search.trim() ? (
        <footer
          className="mt-6 flex flex-wrap gap-3 text-xs text-subtle"
          data-testid="metrics-scoped-footer"
        >
          <span>Scoped links for {search.trim()}:</span>
          <Link to={pathWithQuery(viewToPath('cost'), { workload: search.trim() })} className="text-primary hover:underline">
            Cost
          </Link>
          <Link to={pathWithQuery(viewToPath('health'), { workload: search.trim() })} className="text-primary hover:underline">
            Health
          </Link>
          <Link to={pathWithQuery(viewToPath('fleet'), { workload: search.trim() })} className="text-primary hover:underline">
            Fleet
          </Link>
        </footer>
      ) : null}
    </div>
  );
}

export default withAuroraPage('metrics', MetricsPage);

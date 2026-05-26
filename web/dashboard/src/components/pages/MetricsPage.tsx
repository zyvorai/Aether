// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { ExternalLink } from 'lucide-react';
import { apiFetchSettled, apiTextSettled } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import StatCard from '../StatCard';
import CodeBlock from '../CodeBlock';
import type { ObservabilitySummary } from '../../types/api';

interface ChargebackReport {
  totalMonthlyUsd: number;
  totalSpotMonthlyUsd: number;
  tco36MonthsUsd: number;
  pricingSource: string;
  region: string;
  lines: { workload: string; owner: string; project: string; monthlyUsd: number }[];
}

export default function MetricsPage() {
  const [metrics, setMetrics] = useState('');
  const [summary, setSummary] = useState<ObservabilitySummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useState('');
  const [grafanaUrl, setGrafanaUrl] = useState<string | null>(null);
  const [prometheusUrl, setPrometheusUrl] = useState<string | null>(null);
  const [chargeback, setChargeback] = useState<ChargebackReport | null>(null);

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
  }, []);

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

  if (loading && !metrics && !loadFailed) {
    return <PageLoading rows={6} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Metrics unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search metric names…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      {summary && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
          <StatCard title="API requests" value={Math.round(summary.api_http_requests_total)} color="blue" />
          <StatCard title="Migrations" value={Math.round(summary.migrations_total)} color="orange" />
          <StatCard title="Rollbacks" value={Math.round(summary.migration_rollbacks_total)} color="red" />
          <StatCard
            title="Cluster CPU"
            value={summary.cluster_metrics ? `${summary.cluster_metrics.total_cpu_millicores}m` : '—'}
            color="green"
          />
        </div>
      )}

      {summary && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
          <div className="dash-card">
            <h2 className="text-lg font-semibold text-slate-100 mb-3">Workloads by runtime</h2>
            {runtimeEntries.length > 0 ? (
              <dl className="space-y-2 text-sm">
                {runtimeEntries.map(([runtime, count]) => (
                  <div key={runtime} className="flex justify-between">
                    <dt className="text-slate-400">{runtime}</dt>
                    <dd className="text-slate-200 font-mono">{Math.round(count)}</dd>
                  </div>
                ))}
              </dl>
            ) : (
              <p className="text-sm text-slate-500">No running workload gauges reported yet.</p>
            )}
          </div>
          <div className="dash-card">
            <h2 className="text-lg font-semibold text-slate-100 mb-3">Cluster & Cilium</h2>
            {summary.cluster_metrics ? (
              <dl className="space-y-2 text-sm mb-4">
                <div className="flex justify-between">
                  <dt className="text-slate-400">Scope</dt>
                  <dd className="text-slate-200">{summary.cluster_metrics.scope}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-slate-400">Pods measured</dt>
                  <dd className="text-slate-200">{summary.cluster_metrics.pod_count}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-slate-400">Memory</dt>
                  <dd className="text-slate-200">{summary.cluster_metrics.total_memory_mib} Mi</dd>
                </div>
              </dl>
            ) : (
              <p className="text-sm text-slate-500 mb-4">Pass <code>cluster=</code> to <code>/api/observability/summary</code> for cluster metrics.</p>
            )}
            {summary.cilium ? (
              <dl className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <dt className="text-slate-400">CNI</dt>
                  <dd className="text-slate-200">{summary.cilium.cni}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-slate-400">Egress mode</dt>
                  <dd className="text-slate-200">{summary.cilium.egress_mode}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-slate-400">metrics-server</dt>
                  <dd className="text-slate-200">{summary.cilium.metrics_server ? 'ok' : 'missing'}</dd>
                </div>
              </dl>
            ) : (
              <p className="text-sm text-slate-500">Cilium status unavailable from active cluster.</p>
            )}
            {summary.prometheus_configured && (
              <p className="mt-3 text-xs text-slate-500">
                Prometheus linked — whitelisted queries via <code>/api/observability/prometheus/query</code>
              </p>
            )}
          </div>
        </div>
      )}

      {chargeback && (
        <div className="dash-card mb-6">
          <h2 className="text-lg font-semibold text-slate-100 mb-3">Chargeback (showback)</h2>
          <p className="text-sm text-slate-400 mb-4">
            {chargeback.pricingSource} pricing · {chargeback.region} · fleet ${chargeback.totalMonthlyUsd.toFixed(2)}/mo
            · spot ${chargeback.totalSpotMonthlyUsd.toFixed(2)}/mo · 36-mo TCO ${chargeback.tco36MonthsUsd.toFixed(0)}
          </p>
          {chargeback.lines.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full text-sm text-left text-slate-300">
                <thead className="text-xs uppercase text-slate-500 border-b border-zinc-700">
                  <tr>
                    <th className="py-2 pr-4">Workload</th>
                    <th className="py-2 pr-4">Owner</th>
                    <th className="py-2 pr-4">Project</th>
                    <th className="py-2">$/mo</th>
                  </tr>
                </thead>
                <tbody>
                  {chargeback.lines.slice(0, 12).map((line) => (
                    <tr key={line.workload} className="border-b border-zinc-800/80">
                      <td className="py-2 pr-4 font-mono text-xs">{line.workload}</td>
                      <td className="py-2 pr-4">{line.owner}</td>
                      <td className="py-2 pr-4">{line.project}</td>
                      <td className="py-2">${line.monthlyUsd.toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p className="text-sm text-slate-500">No deployed workloads with readable specs for chargeback.</p>
          )}
        </div>
      )}

      {(grafanaUrl || prometheusUrl) && (
        <div className="dash-card mb-6 flex flex-wrap items-center justify-between gap-4">
          <p className="text-sm text-slate-400">External observability stack linked to this API.</p>
          <div className="flex flex-wrap gap-3">
            {grafanaUrl && (
              <a
                href={grafanaUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 rounded-xl border border-aether/40 bg-aether/10 px-4 py-2 text-sm text-aether hover:bg-aether/20"
              >
                Open Grafana <ExternalLink size={14} />
              </a>
            )}
            {prometheusUrl && (
              <a
                href={prometheusUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800"
              >
                Prometheus <ExternalLink size={14} />
              </a>
            )}
          </div>
        </div>
      )}

      <div className="dash-card">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-slate-100">Prometheus metrics</h2>
          <span className="text-xs text-slate-500">{lineCount} metric lines</span>
        </div>
        <CodeBlock title="prometheus">{filteredMetrics || 'No metrics match your search.'}</CodeBlock>
      </div>
    </div>
  );
}

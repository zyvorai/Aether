import { useState, useEffect, useCallback, useMemo } from 'react';
import { ExternalLink } from 'lucide-react';
import { apiFetchSettled, apiTextSettled } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import CodeBlock from '../CodeBlock';

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
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useState('');
  const [grafanaUrl, setGrafanaUrl] = useState<string | null>(null);
  const [chargeback, setChargeback] = useState<ChargebackReport | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [metricsRes, serverRes, chargebackRes] = await Promise.all([
      apiTextSettled('/metrics'),
      apiFetchSettled<Record<string, unknown>>('/server'),
      apiFetchSettled<ChargebackReport>('/cost/chargeback?provider=aws'),
    ]);
    if (!metricsRes.ok) {
      setLoadFailed(true);
      setMetrics('');
      setGrafanaUrl(null);
      setChargeback(null);
    } else {
      setMetrics(metricsRes.data);
      if (serverRes.ok) {
        const integrations = serverRes.data?.integrations as Record<string, unknown> | undefined;
        const url = integrations?.grafana_url;
        setGrafanaUrl(typeof url === 'string' ? url : null);
      } else {
        setGrafanaUrl(null);
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

      {grafanaUrl && (
        <div className="dash-card mb-6 flex items-center justify-between gap-4">
          <p className="text-sm text-slate-400">Open Grafana for dashboards and alerting.</p>
          <a
            href={grafanaUrl}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-2 rounded-xl border border-aether/40 bg-aether/10 px-4 py-2 text-sm text-aether hover:bg-aether/20"
          >
            Open Grafana <ExternalLink size={14} />
          </a>
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

// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Globe, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { formatPercent } from '../utils/formatters';
import Badge from './Badge';
import type { MultiCloudPostureReport } from '../types/api';
import GlassSection from './GlassSection';

export default function MultiCloudPanel() {
  const [report, setReport] = useState<MultiCloudPostureReport | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<MultiCloudPostureReport>('/intelligence/multicloud/posture');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <GlassSection
      accent="blue"
      testId="multicloud-panel"
      title="Multi-Cloud Posture"
      subtitle="Federated cluster reachability, anomaly signals, and placement scores."
      icon={<Globe className="h-5 w-5 text-teal-400" />}
      actions={
        <button
          type="button"
          onClick={() => void load()}
          className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
        >
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          Refresh
        </button>
      }
    >
      {report ? (
        <>
          <div className="mb-6 flex flex-wrap items-center gap-3">
            <Badge
              text={report.federation_enabled ? 'Federation active' : 'Single cluster'}
              variant={report.federation_enabled ? 'green' : 'muted'}
            />
            <span className="text-xs text-slate-500">
              {report.reachable_clusters}/{report.clusters.length} clusters reachable · {report.fleet_workloads} workloads
            </span>
          </div>

          <div className="mb-6 grid gap-3 sm:grid-cols-3">
            {report.clusters.slice(0, 6).map((cluster) => (
              <div key={cluster.cluster} className="rounded-2xl border border-slate-800/70 bg-slate-950/40 p-4">
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium text-white">{cluster.cluster}</span>
                  <Badge text={cluster.reachable ? 'up' : 'down'} variant={cluster.reachable ? 'green' : 'red'} />
                </div>
                <p className="mt-2 text-xs text-slate-500">{cluster.server ?? 'local context'}</p>
                <div className="mt-3 flex flex-wrap gap-2 text-xs text-slate-400">
                  <span>Score {formatPercent(cluster.score * 100, 0)}</span>
                  {cluster.anomaly_count > 0 ? <span className="text-amber-300">{cluster.anomaly_count} anomalies</span> : null}
                </div>
              </div>
            ))}
          </div>

          <ul className="space-y-2">
            {report.recommended_actions.map((line) => (
              <li key={line} className="rounded-lg border border-slate-800 px-3 py-2 text-sm text-slate-300">
                {line}
              </li>
            ))}
          </ul>
        </>
      ) : loading ? (
        <div className="flex items-center gap-2 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading multi-cloud posture…
        </div>
      ) : null}
    </GlassSection>
  );
}

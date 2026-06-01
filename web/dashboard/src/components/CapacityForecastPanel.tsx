// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router';
import { Gauge, Loader2, RefreshCw, TrendingUp } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { formatPercent } from '../utils/formatters';
import { viewToPath } from '../utils/dashboardRoutes';
import type { CommandCenterBriefingData } from './CommandCenterBriefing';
import type { PredictionReport, RuntimeUtilization } from '../types/api';
import GlassSection from './GlassSection';

function utilTone(value: number): string {
  if (value >= 0.85) return 'text-red-300';
  if (value >= 0.7) return 'text-amber-200';
  return 'text-emerald-300';
}

function UtilBar({ label, value }: { label: string; value: number }) {
  const pct = Math.round(value * 100);
  return (
    <div>
      <div className="mb-1 flex items-center justify-between text-xs">
        <span className="text-slate-500">{label}</span>
        <span className={utilTone(value)}>{pct}%</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-slate-800">
        <div
          className={`h-full rounded-full transition-all ${value >= 0.85 ? 'bg-red-500' : value >= 0.7 ? 'bg-amber-500' : 'bg-emerald-500'}`}
          style={{ width: `${Math.min(100, pct)}%` }}
        />
      </div>
    </div>
  );
}

export default function CapacityForecastPanel() {
  const [loading, setLoading] = useState(true);
  const [predictions, setPredictions] = useState<PredictionReport | null>(null);
  const [utilization, setUtilization] = useState<RuntimeUtilization[]>([]);
  const [briefing, setBriefing] = useState<CommandCenterBriefingData | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [pred, util, brief] = await Promise.all([
      apiFetch<PredictionReport>('/intelligence/predictions'),
      apiFetch<RuntimeUtilization[]>('/scheduler/utilization'),
      apiFetch<CommandCenterBriefingData>('/command-center/briefing'),
    ]);
    setPredictions(pred);
    setUtilization(util ?? []);
    setBriefing(brief);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const topRisks = useMemo(
    () =>
      (predictions?.predictions ?? [])
        .filter((p) => p.risk_level === 'high' || p.risk_level === 'critical')
        .slice(0, 4),
    [predictions],
  );

  const avgCpu =
    utilization.length > 0
      ? utilization.reduce((sum, u) => sum + u.cpu_utilization, 0) / utilization.length
      : 0;
  const avgMem =
    utilization.length > 0
      ? utilization.reduce((sum, u) => sum + u.memory_utilization, 0) / utilization.length
      : 0;

  return (
    <GlassSection
      accent="blue"
      testId="capacity-forecast-panel"
      title="Capacity Forecast"
      subtitle="Scheduler utilization + failure predictions + saturation horizon."
      icon={<Gauge className="h-5 w-5 text-amber-400" />}
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
      <div className="grid gap-6 lg:grid-cols-3">
        <div className="space-y-4 glass-panel-card p-4">
          <p className="text-xs font-semibold uppercase tracking-wider text-slate-500">Runtime utilization</p>
          <UtilBar label="Fleet CPU (avg)" value={avgCpu} />
          <UtilBar label="Fleet memory (avg)" value={avgMem} />
          {utilization.slice(0, 3).map((u) => (
            <div key={u.runtime} className="text-xs text-slate-500">
              {u.runtime}: {Math.round(u.cpu_utilization * 100)}% CPU · {u.workload_count}/{u.max_workloads} workloads
            </div>
          ))}
        </div>

        <div className="space-y-3 glass-panel-card p-4">
          <p className="text-xs font-semibold uppercase tracking-wider text-slate-500">Fleet risk</p>
          <div className="flex items-end gap-2">
            <span className="text-3xl font-semibold text-white">
              {formatPercent((predictions?.fleet_risk_score ?? 0) * 100, 0)}
            </span>
            <TrendingUp className="mb-1 h-4 w-4 text-amber-400" />
          </div>
          {topRisks.length === 0 ? (
            <p className="text-sm text-slate-500">No elevated workload risks detected.</p>
          ) : (
            <ul className="space-y-2">
              {topRisks.map((row) => (
                <li key={row.workload} className="rounded-lg border border-slate-800 px-3 py-2 text-sm">
                  <Link
                    to={`${viewToPath('workloads')}?workload=${encodeURIComponent(row.workload)}`}
                    className="font-medium text-aether hover:underline"
                  >
                    {row.workload}
                  </Link>
                  <p className="text-xs text-slate-500">{row.predictions[0]?.reason ?? row.risk_level}</p>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="space-y-3 glass-panel-card p-4">
          <p className="text-xs font-semibold uppercase tracking-wider text-slate-500">Saturation horizon</p>
          {(briefing?.capacity_risks ?? []).length === 0 ? (
            <p className="text-sm text-slate-500">No capacity saturation signals in the next 30 days.</p>
          ) : (
            <ul className="space-y-2">
              {briefing!.capacity_risks.map((risk) => (
                <li key={`${risk.resource}-${risk.summary}`} className="rounded-lg border border-amber-500/20 bg-amber-500/5 px-3 py-2">
                  <div className="flex items-center justify-between gap-2 text-sm">
                    <span className="font-medium text-amber-100">{risk.resource}</span>
                    <span className="text-xs text-amber-200/80">{risk.days_remaining}d</span>
                  </div>
                  <p className="mt-1 text-xs text-amber-100/80">{risk.summary}</p>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </GlassSection>
  );
}

// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { ArrowRight, DollarSign, Loader2, RefreshCw, Sparkles } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { formatPercent, formatUSD } from '../utils/formatters';
import { viewToPath } from '../utils/dashboardRoutes';
import Badge from './Badge';
import type { CostApplyReport, CostOptimizeReport } from '../types/api';

function riskVariant(risk: string): 'green' | 'yellow' | 'red' | 'muted' {
  const l = risk.toLowerCase();
  if (l === 'low') return 'green';
  if (l === 'medium') return 'yellow';
  return 'red';
}

export default function CostIntelligencePanel() {
  const [report, setReport] = useState<CostOptimizeReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<CostOptimizeReport>('/intelligence/cost-optimize');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const totalSavings =
    report?.recommendations.reduce((sum, r) => sum + r.savings_monthly_usd, 0) ?? 0;

  async function applyPatches(dryRun: boolean) {
    setApplying(true);
    const res = await apiPost<CostApplyReport>('/intelligence/cost-optimize/apply', { dry_run: dryRun });
    setApplying(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `Cost dry-run: ${res.data?.applied.length ?? 0} patches`
              : `Cost applied ${res.data?.applied.length ?? 0} right-size patches`,
            type: 'success',
          },
        }),
      );
    }
  }

  async function enforceBudget(dryRun: boolean) {
    setApplying(true);
    const res = await apiPost<{ actions: Array<{ action: string }> }>('/intelligence/intent/budget/enforce', {
      dry_run: dryRun,
    });
    setApplying(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: `Budget enforce: ${res.data?.actions.length ?? 0} actions`,
            type: 'info',
          },
        }),
      );
    }
  }

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="cost-intelligence-panel">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Sparkles className="h-5 w-5 text-emerald-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Cost Intelligence</h2>
            <p className="text-sm text-slate-400">
              FinOps recommendations from utilization profiles and runtime placement.
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => void enforceBudget(true)}
            disabled={applying}
            className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-200 hover:border-amber-400/50 disabled:opacity-60"
            data-testid="intent-budget-enforce"
          >
            Enforce budget
          </button>
          <button
            type="button"
            onClick={() => void applyPatches(true)}
            disabled={applying || !(report?.recommendations.length ?? 0)}
            className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-200 hover:border-emerald-400/50 disabled:opacity-60"
            data-testid="cost-apply-dry-run"
          >
            Dry-run right-size
          </button>
          <Link to={viewToPath('intelligence') + '?tab=cost'} className="text-xs text-aether hover:underline">
            Full report →
          </Link>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>
      </div>

      <div className="mb-6 grid gap-3 sm:grid-cols-3">
        <div className="glass-metric-card">
          <DollarSign className="mb-2 h-4 w-4 text-emerald-400" />
          <div className="text-2xl font-semibold text-emerald-300">{formatUSD(totalSavings)}</div>
          <div className="text-xs text-slate-500">Potential savings / mo</div>
        </div>
        <div className="glass-metric-card">
          <div className="text-2xl font-semibold text-white">
            {formatPercent(report?.total_potential_savings_pct ?? 0, 1)}
          </div>
          <div className="text-xs text-slate-500">Fleet savings potential</div>
        </div>
        <div className="glass-metric-card">
          <div className="text-2xl font-semibold text-white">{report?.recommendations.length ?? 0}</div>
          <div className="text-xs text-slate-500">Optimization opportunities</div>
        </div>
      </div>

      {!report?.recommendations.length ? (
        <p className="text-sm text-slate-500">No cost optimizations identified — fleet placement looks efficient.</p>
      ) : (
        <ul className="space-y-3">
          {report.recommendations.slice(0, 5).map((rec) => (
            <li
              key={rec.workload}
              className="flex flex-wrap items-start justify-between gap-3 rounded-2xl border border-slate-800/70 bg-slate-950/40 px-4 py-3"
            >
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <Link
                    to={`${viewToPath('workloads')}?workload=${encodeURIComponent(rec.workload)}`}
                    className="font-medium text-aether hover:underline"
                  >
                    {rec.workload}
                  </Link>
                  <Badge text={`${rec.risk} risk`} variant={riskVariant(rec.risk)} />
                </div>
                <p className="mt-1 text-sm text-slate-400">{rec.reason}</p>
                <p className="mt-1 text-xs text-slate-500">
                  {rec.current_runtime} → {rec.suggested_runtime} · save {formatPercent(rec.savings_pct, 0)}
                </p>
              </div>
              <div className="text-right">
                <div className="text-lg font-semibold text-emerald-300">{formatUSD(rec.savings_monthly_usd)}</div>
                <Link
                  to={`${viewToPath('migrations')}?workload=${encodeURIComponent(rec.workload)}`}
                  className="mt-1 inline-flex items-center gap-1 text-xs text-aether hover:underline"
                >
                  Plan migration <ArrowRight className="h-3 w-3" />
                </Link>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

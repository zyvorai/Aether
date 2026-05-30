// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { Loader2, MapPin, RefreshCw } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { formatPercent } from '../utils/formatters';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import Badge from './Badge';
import type { EvolutionExecuteReport, EvolutionStatus } from '../types/api';

export default function AutonomousPlacementPanel() {
  const [status, setStatus] = useState<EvolutionStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<EvolutionStatus>('/intelligence/autonomous/placement');
    setStatus(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const candidates = (status?.workloads ?? []).filter((w) => w.recommended_runtime !== w.current_runtime);

  async function runEvolution(dryRun: boolean) {
    if (!dryRun) {
      const ok = window.confirm('Execute autonomous migrations? Requires evolution autonomy policy.');
      if (!ok) return;
    }
    setExecuting(true);
    const res = await apiPost<EvolutionExecuteReport>('/intelligence/evolution/execute', {
      dry_run: dryRun,
      max_actions: 5,
    });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `Evolution dry-run: ${res.data?.executed.length ?? 0} would migrate`
              : `Evolution executed ${res.data?.executed.length ?? 0} migrations`,
            type: 'success',
          },
        }),
      );
      void load();
    }
  }

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="autonomous-placement-panel">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <MapPin className="h-5 w-5 text-emerald-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Autonomous Placement</h2>
            <p className="text-sm text-slate-400">
              Fleet runtime evolution — auto-eligible migrations ranked by improvement confidence.
            </p>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void runEvolution(true)}
            disabled={executing}
            className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-200 hover:border-emerald-400/50 disabled:opacity-60"
            data-testid="evolution-dry-run"
          >
            Dry-run evolution
          </button>
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
          <div className="text-2xl font-semibold text-white">{status?.workloads.length ?? 0}</div>
          <div className="text-xs text-slate-500">Tracked workloads</div>
        </div>
        <div className="glass-metric-card">
          <div className="text-2xl font-semibold text-emerald-300">{candidates.length}</div>
          <div className="text-xs text-slate-500">Migration candidates</div>
        </div>
        <div className="glass-metric-card">
          <div className="text-2xl font-semibold text-violet-300">
            {status?.workloads.filter((w) => w.auto_eligible).length ?? 0}
          </div>
          <div className="text-xs text-slate-500">Auto-eligible</div>
        </div>
      </div>

      {!candidates.length ? (
        <p className="text-sm text-slate-500">Fleet placement is optimal — no runtime shifts recommended.</p>
      ) : (
        <ul className="space-y-3">
          {candidates.slice(0, 8).map((row) => (
            <li
              key={row.workload}
              className="flex flex-wrap items-start justify-between gap-3 rounded-2xl border border-slate-800/70 bg-slate-950/40 px-4 py-3"
            >
              <div>
                <div className="flex flex-wrap items-center gap-2">
                  <Link
                    to={pathWithQuery(viewToPath('migrations'), { workload: row.workload })}
                    className="font-medium text-aether hover:underline"
                  >
                    {row.workload}
                  </Link>
                  {row.auto_eligible ? <Badge text="auto-eligible" variant="green" /> : null}
                </div>
                <p className="mt-1 text-sm text-slate-400">
                  {row.current_runtime} → {row.recommended_runtime} · +{formatPercent(row.improvement_pct, 0)} improvement
                </p>
                <p className="mt-1 text-xs text-slate-500">{row.reasons[0] ?? 'Scoring engine recommendation'}</p>
              </div>
              <div className="text-right text-sm text-slate-400">
                {formatPercent(row.confidence * 100, 0)} conf.
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

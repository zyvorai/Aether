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
import GlassSection from './GlassSection';

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
        <GlassSection
      accent="purple"
      testId="autonomous-placement-panel"
      title="Autonomous Placement"
      subtitle="Fleet runtime evolution — auto-eligible migrations ranked by improvement confidence."
      icon={<MapPin className="h-5 w-5 text-emerald-400" />}
      actions={<div className="flex flex-wrap gap-2">
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
            className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-muted hover:border-primary/40"
          >
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          Refresh
          </button>
        </div>}
    ><div className="mb-6 grid gap-3 sm:grid-cols-3">
        <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
          <div className="text-2xl font-semibold text-foreground">{status?.workloads.length ?? 0}</div>
          <div className="text-xs text-subtle">Tracked workloads</div>
        </div>
        <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
          <div className="text-2xl font-semibold text-emerald-300">{candidates.length}</div>
          <div className="text-xs text-subtle">Migration candidates</div>
        </div>
        <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
          <div className="text-2xl font-semibold text-violet-300">
            {status?.workloads.filter((w) => w.auto_eligible).length ?? 0}
          </div>
          <div className="text-xs text-subtle">Auto-eligible</div>
        </div>
      </div>

      {!candidates.length ? (
        <p className="text-sm text-subtle">Fleet placement is optimal — no runtime shifts recommended.</p>
      ) : (
        <ul className="space-y-3">
          {candidates.slice(0, 8).map((row) => (
            <li
              key={row.workload}
              className="flex flex-wrap items-start justify-between gap-3 glass px-4 py-3"
            >
              <div>
                <div className="flex flex-wrap items-center gap-2">
                  <Link
                    to={pathWithQuery(viewToPath('migrations'), { workload: row.workload })}
                    className="font-medium text-primary hover:underline"
                  >
                    {row.workload}
                  </Link>
                  {row.auto_eligible ? <Badge text="auto-eligible" variant="green" /> : null}
                </div>
                <p className="mt-1 text-sm text-muted">
                  {row.current_runtime} → {row.recommended_runtime} · +{row.improvement_pct.toFixed(0)}% improvement
                </p>
                <p className="mt-1 text-xs text-subtle">{row.reasons[0] ?? 'Scoring engine recommendation'}</p>
              </div>
              <div className="text-right text-sm text-muted">
                {formatPercent(row.confidence, 0)} conf.
              </div>
            </li>
          ))}
        </ul>
      )}
    </GlassSection>
  );
}
